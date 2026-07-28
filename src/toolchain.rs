//! Bundled toolchain resolution for the Nail IDE.
//!
//! A Nail release installs as one immutable bundle at /opt/nail containing
//! the IDE, a pinned Rust toolchain, vendored crate sources, the nail crate
//! source, and a pre-warmed build cache. When the bundle is present, builds
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

/// Default install location. Fixed on purpose: cargo's fingerprints embed
/// absolute paths, so a fixed path lets the pre-warmed cache shipped in the
/// bundle stay valid on every machine.
const DEFAULT_BUNDLE_ROOT: &str = "/opt/nail";

pub struct BundledToolchain {
    root: PathBuf,
}

impl BundledToolchain {
    /// Detect an installed bundle. NAIL_HOME overrides the default root
    /// (used by the bundle build itself and by tests).
    pub fn detect() -> Option<Self> {
        let root = std::env::var_os("NAIL_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEFAULT_BUNDLE_ROOT));
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
