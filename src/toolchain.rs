//! Bundled toolchain resolution for the Nail IDE.
//!
//! A Nail release installs as one immutable bundle at
//! /opt/nail/versions/<version> containing the IDE, a pinned Rust toolchain,
//! vendored crate sources, the nail crate source, and a pre-warmed build
//! cache. The launcher installs and runs them. When the bundle is present, builds
//! use its cargo with a scrubbed environment so nothing on the user's machine
//! (rustup, RUSTFLAGS, crates.io) can affect or break compilation. Without a
//! bundle (development checkouts), builds fall back to the system cargo.
//!
//! Layout under the bundle root:
//!   bin/          nail (IDE), nailc
//!   toolchain/    pinned Rust dist (bin/rustc, bin/cargo, rust-lld, ...)
//!   cargo-home/   config.toml (vendored sources, offline, musl target,
//!                 rust-lld linker) and the vendored crates + registry cache
//!   nail/         nail crate source that generated programs depend on
//!   cache/        shared warm target directory, owned by the installing user

use std::path::{Path, PathBuf};
use std::process::Command;

/// The one target user programs are built for: fully static via the
/// toolchain's own musl libc and rust-lld, so linking needs zero system files.
pub const BUNDLE_TARGET: &str = "x86_64-unknown-linux-musl";

/// Where the launcher keeps installed versions. Each one lives at
/// `<VERSION_STORE>/<version>` and is built at exactly that path, because
/// cargo's fingerprints embed absolute paths and a bundle's pre-warmed cache
/// is only valid at the path it was warmed at. The version is known when the
/// bundle is built, so a per-version path is still a fixed path.
pub const VERSION_STORE: &str = "/opt/nail/versions";

pub struct BundledToolchain {
    root: PathBuf,
}

impl BundledToolchain {
    /// Detect the bundle this binary belongs to. Since many versions are
    /// installed side by side, the root cannot be a constant: it is found from
    /// the running executable's own location (`<root>/bin/nailc`), so a binary
    /// always builds with the toolchain it shipped with. NAIL_HOME overrides
    /// it, which is what the bundle build itself and the tests use. In a
    /// development checkout nothing matches and builds fall back to the system
    /// cargo, exactly as before.
    pub fn detect() -> Option<Self> {
        let root = match std::env::var_os("NAIL_HOME") {
            Some(home) => PathBuf::from(home),
            None => std::env::current_exe().ok()?.parent()?.parent()?.to_path_buf(),
        };
        let toolchain = Self { root };
        if toolchain.cargo_binary().is_file() && toolchain.cargo_home().is_dir() && toolchain.nail_crate_path().join("Cargo.toml").is_file() {
            Some(toolchain)
        } else {
            None
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path generated Cargo.toml files use for the nail crate dependency.
    pub fn nail_crate_path(&self) -> PathBuf {
        self.root.join("nail")
    }

    /// All projects share the bundle's target directory (see cargo_command),
    /// so the pre-warmed dependency cache shipped in the bundle applies to
    /// every build. This is where cargo puts the release binary in it.
    pub fn built_binary_path(&self, package_name: &str) -> PathBuf {
        self.root.join("cache/target").join(BUNDLE_TARGET).join("release").join(package_name)
    }

    fn cargo_binary(&self) -> PathBuf {
        self.root.join("toolchain/bin/cargo")
    }

    fn cargo_home(&self) -> PathBuf {
        self.root.join("cargo-home")
    }

    /// A cargo Command with a scrubbed environment: only the bundled
    /// toolchain is on PATH and only the bundle's cargo-home is consulted,
    /// so user rustup installs, RUSTFLAGS, CARGO_* variables and crates.io
    /// cannot influence the build. All build configuration (vendored
    /// sources, offline mode, target, linker) lives in the bundle's
    /// cargo-home/config.toml rather than in code.
    pub fn cargo_command(&self) -> Command {
        let mut command = Command::new(self.cargo_binary());
        command.env_clear();
        command.env("PATH", format!("{}:/usr/bin:/bin", self.root.join("toolchain/bin").display()));
        command.env("CARGO_HOME", self.cargo_home());
        command.env("CARGO_TARGET_DIR", self.root.join("cache/target"));
        if let Some(home) = std::env::var_os("HOME") {
            command.env("HOME", home);
        }
        command
    }
}

/// Guards on the things a release has to get right, which are exactly the
/// things that cannot be noticed until a release is built. Each of these
/// corresponds to a bug that shipped once.
#[cfg(test)]
mod release_guards {
    use std::path::{Path, PathBuf};

    fn repo_root() -> PathBuf {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    }

    fn nail_files(root: &Path, found: &mut Vec<PathBuf>) {
        const SKIP: [&str; 4] = [".git", "target", "vendor", "node_modules"];
        let entries = match std::fs::read_dir(root) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() && !SKIP.contains(&name.as_str()) => nail_files(&path, found),
                // `.nail` itself is the editor's settings file, not source.
                Ok(kind) if kind.is_file() && name.ends_with(".nail") && !name.starts_with('.') => found.push(path),
                _ => {}
            }
        }
    }

    /// The compiler refuses a file with no version line, so a `.nail` file
    /// anywhere in this repository that lacks one is a file that cannot be
    /// compiled. The bundle's own warmup fixture was missing one, and nothing
    /// noticed until a release build failed on it.
    #[test]
    fn every_nail_file_carries_a_version_line() {
        let mut files = Vec::new();
        nail_files(&repo_root(), &mut files);
        assert!(files.len() > 100, "expected to find the repository's .nail files, found {}", files.len());

        let unstamped: Vec<String> = files
            .iter()
            .filter(|path| {
                let source = std::fs::read_to_string(path).unwrap_or_default();
                crate::version_line::scan_header(source.as_bytes()).pin.is_none()
            })
            .map(|path| path.strip_prefix(repo_root()).unwrap_or(path).display().to_string())
            .collect();

        assert!(unstamped.is_empty(), "these .nail files have no version line, so nailc will refuse them:\n  {}", unstamped.join("\n  "));
    }

    /// `bundle/build_bundle.sh` copies a subset of the repository into the
    /// bundle as the nail crate source, and every program a user builds
    /// compiles that copy. So anything the *library* embeds with
    /// `include_bytes!` or `include_str!` has to be inside what gets copied,
    /// or every user build fails. An embedded font in `assets/` was missing
    /// once and only a release build revealed it.
    ///
    /// Only the library counts. The binaries ship already compiled, so what
    /// they embed is nobody's problem after the release machine is done.
    #[test]
    fn what_the_library_embeds_is_inside_what_the_bundle_copies() {
        let script = std::fs::read_to_string(repo_root().join("bundle/build_bundle.sh")).expect("the bundle build script should exist");

        let mut sources = Vec::new();
        for module in library_modules() {
            let file = repo_root().join("src").join(format!("{}.rs", module));
            if file.is_file() {
                sources.push(file);
            }
            let directory = repo_root().join("src").join(&module);
            if directory.is_dir() {
                collect_rust_sources(&directory, &mut sources);
            }
        }
        sources.push(repo_root().join("src/lib.rs"));
        assert!(sources.len() > 5, "expected to find the library's sources, found {}", sources.len());

        let mut escaping: Vec<String> = Vec::new();
        for source in &sources {
            let text = std::fs::read_to_string(source).unwrap_or_default();
            for macro_name in ["include_bytes!", "include_str!"] {
                for piece in text.split(macro_name).skip(1) {
                    let literal: String = piece.trim_start().trim_start_matches('(').trim_start().trim_start_matches('"').chars().take_while(|character| *character != '"').collect();
                    if !literal.starts_with("../") {
                        continue;
                    }
                    // What matters is the first thing named once the climbing
                    // out of src/ stops. Landing back inside src/ is fine,
                    // since all of src/ is copied.
                    let landed = literal.trim_start_matches("../");
                    if landed.starts_with("src/") {
                        continue;
                    }
                    let target = landed.split('/').next().unwrap_or("").to_string();
                    if !target.is_empty() && !escaping.contains(&target) {
                        escaping.push(target);
                    }
                }
            }
        }

        for target in &escaping {
            assert!(
                script.contains(&format!("$REPO/{}", target)),
                "the library embeds {} from outside src/, but bundle/build_bundle.sh never copies it into $ROOT/nail, so every user build would fail",
                target
            );
        }
    }

    /// The modules `lib.rs` declares, which is what a user program compiles.
    fn library_modules() -> Vec<String> {
        let lib = std::fs::read_to_string(repo_root().join("src/lib.rs")).expect("src/lib.rs should exist");
        return lib
            .lines()
            .filter_map(|line| line.trim().strip_prefix("pub mod ").or_else(|| line.trim().strip_prefix("mod ")))
            .filter_map(|rest| rest.split(';').next())
            .map(|name| name.trim().to_string())
            .collect();
    }

    fn collect_rust_sources(root: &Path, found: &mut Vec<PathBuf>) {
        let entries = match std::fs::read_dir(root) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => collect_rust_sources(&path, found),
                Ok(kind) if kind.is_file() && path.extension().is_some_and(|extension| extension == "rs") => found.push(path),
                _ => {}
            }
        }
    }
}
