//! The version version line: the one line of a Nail file that says which compiler
//! wrote it.
//!
//! This module is the frozen contract between a source file and Hammer. Its
//! grammar can never change, because a Hammer built today has to read a file
//! written in ten years, and a compiler from ten years ago has to read a file
//! written today:
//!
//! ```text
//! file    = [ BOM ] [ shebang LF ] version line ( LF | EOF )
//! shebang = "#!" *( byte except LF )
//! version line  = "nail" SP ( "latest" / version ) [ CR ]
//! version = num "." num "." num [ "-" pre ]
//! num     = 1*9DIGIT
//! pre     = 1*( ALPHA / DIGIT / "-" / "." )
//! ```
//!
//! Three states, deliberately distinct. `nail 0.3.1` is archived and frozen,
//! and the IDE maintains it. `nail latest` tracks whatever is installed, and
//! the IDE leaves it alone. No version line at all is undecided, and the IDE stamps
//! a concrete version on save.
//!
//! There are no ranges. A partial version like `0.3` or a constraint like
//! `^0.3` cannot be written, because accepting one would mean writing
//! resolution rules, and resolution rules are where bit rot lives. The parser
//! shape enforces that, rather than a check somewhere downstream.
//!
//! Parsing takes bytes rather than text, and reads only the head of a file. A
//! file whose body is invalid UTF-8, or which uses syntax from a release that
//! does not exist yet, must still resolve to a version and launch.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// How much of a file's head is ever examined. The version line lives in the first
/// two lines, so anything past this is somebody else's problem.
pub const HEAD_BYTES: usize = 4096;

/// Nine digits per field: cannot overflow u32, and is far past any version a
/// human would write.
const MAX_DIGITS: usize = 9;

const BOM: &[u8] = b"\xEF\xBB\xBF";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    /// A `-dev` style suffix marks a build that was never published. Hammer
    /// refuses to fetch one, so these only ever name a locally built compiler.
    pub prerelease: Option<String>,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Version { major, minor, patch, prerelease: None }
    }

    /// A prerelease was built locally and was never published, so Hammer must
    /// not try to download it.
    pub fn is_prerelease(&self) -> bool {
        return self.prerelease.is_some();
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let numeric = (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch));
        if numeric != Ordering::Equal {
            return numeric;
        }
        // A prerelease comes before the release it leads to, matching semver's
        // spirit without implementing its full precedence rules.
        match (&self.prerelease, &other.prerelease) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(left), Some(right)) => left.cmp(right),
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return Some(self.cmp(other));
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(prerelease) = &self.prerelease {
            write!(formatter, "-{}", prerelease)?;
        }
        return Ok(());
    }
}

impl FromStr for Version {
    type Err = ();

    /// Parses a bare version, with no `nail ` prefix. Used for directory
    /// names, command line arguments and the reply from `/versions/latest`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bytes = text.as_bytes();
        let (version, rest) = take_version(bytes).ok_or(())?;
        if rest.is_empty() {
            Ok(version)
        } else {
            Err(())
        }
    }
}

/// What a file's first line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pin {
    /// `nail latest`: the newest version installed on this machine. Never
    /// reaches the network, so opening a file can never trigger a download of
    /// a compiler the user did not ask for.
    Latest,
    Exact(Version),
}

impl fmt::Display for Pin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pin::Latest => write!(formatter, "latest"),
            Pin::Exact(version) => write!(formatter, "{}", version),
        }
    }
}

impl FromStr for Pin {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text == "latest" {
            return Ok(Pin::Latest);
        }
        return text.parse::<Version>().map(Pin::Exact);
    }
}

/// The head of a file, decomposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// What line one asked for, if it asked for anything.
    pub pin: Option<Pin>,
    /// Bytes of BOM, shebang and version line to skip before the source proper.
    pub byte_len: usize,
    /// Lines those bytes occupied, so spans after them stay truthful.
    pub lines: u32,
}

/// Reads the version line from the head of a source file. Pass at most
/// [`HEAD_BYTES`]; passing the whole file also works and costs nothing extra.
///
/// A malformed first line reads as unpinned rather than as an error, on
/// purpose. An unparseable line one might be legitimate syntax from a release
/// that does not exist yet, and refusing to launch would be the worst possible
/// way to fail.
pub fn read_version_line(head: &[u8]) -> Option<Pin> {
    return scan_header(head).pin;
}

/// Decomposes the head of a file into its version line and the byte and line offsets
/// of the source proper.
pub fn scan_header(head: &[u8]) -> Header {
    let mut offset = 0;
    let mut lines = 0;

    if head.starts_with(BOM) {
        offset += BOM.len();
    }

    // A shebang may sit above the version line, because `#!/usr/bin/env nail`
    // scripts would otherwise fight the version line for line one. Nothing else may.
    if head[offset..].starts_with(b"#!") {
        match line_end(&head[offset..]) {
            Some(length) => {
                offset += length;
                lines += 1;
            }
            // A shebang with no newline after it is the whole file. There is
            // no room left for a version line.
            None => return Header { pin: None, byte_len: head.len(), lines: 1 },
        }
    }

    let after_shebang = offset;
    let rest = match head[offset..].strip_prefix(b"nail ") {
        Some(rest) => rest,
        None => return Header { pin: None, byte_len: after_shebang, lines },
    };

    let (pin, rest) = match take_pin(rest) {
        Some(found) => found,
        None => return Header { pin: None, byte_len: after_shebang, lines },
    };

    let rest = rest.strip_prefix(b"\r").unwrap_or(rest);
    let consumed = head.len() - rest.len();
    match rest.first() {
        Some(b'\n') => Header { pin: Some(pin), byte_len: consumed + 1, lines: lines + 1 },
        // A version line with nothing after it is a file with no source in it.
        None => Header { pin: Some(pin), byte_len: consumed, lines: lines + 1 },
        // Anything else on line one means this was never a version line.
        Some(_) => Header { pin: None, byte_len: after_shebang, lines },
    }
}

/// Splits a source file into the part the lexer should see and the number of
/// header lines skipped, so token spans still name the right lines.
pub fn strip_header(source: &str) -> (&str, u32) {
    let header = scan_header(source.as_bytes());
    // Every cut is at an ASCII newline or the end of the ASCII BOM, so the
    // offset is always a character boundary.
    return (&source[header.byte_len..], header.lines);
}

/// Rewrites a file's version line, or inserts one if it has none. Any shebang stays
/// on top. This is how the IDE maintains the pin on save and how `nailc
/// --stamp` migrates a file.
pub fn stamp(source: &str, pin: &Pin) -> String {
    let bytes = source.as_bytes();
    let header = scan_header(bytes);

    // The BOM and any shebang line are kept verbatim on top of the new version line.
    let mut keep = if bytes.starts_with(BOM) { BOM.len() } else { 0 };
    if bytes[keep..].starts_with(b"#!") {
        keep += line_end(&bytes[keep..]).unwrap_or(bytes.len() - keep);
    }

    let mut out = String::with_capacity(source.len() + 16);
    out.push_str(&source[..keep]);
    if keep > 0 && !source[..keep].ends_with('\n') {
        out.push('\n');
    }
    out.push_str("nail ");
    out.push_str(&pin.to_string());
    out.push('\n');
    out.push_str(&source[header.byte_len.max(keep)..]);
    return out;
}

fn take_pin(bytes: &[u8]) -> Option<(Pin, &[u8])> {
    if let Some(rest) = bytes.strip_prefix(b"latest") {
        return Some((Pin::Latest, rest));
    }
    let (version, rest) = take_version(bytes)?;
    return Some((Pin::Exact(version), rest));
}

fn take_version(bytes: &[u8]) -> Option<(Version, &[u8])> {
    let (major, rest) = take_number(bytes)?;
    let rest = rest.strip_prefix(b".")?;
    let (minor, rest) = take_number(rest)?;
    let rest = rest.strip_prefix(b".")?;
    let (patch, rest) = take_number(rest)?;

    let (prerelease, rest) = match rest.strip_prefix(b"-") {
        Some(tail) => {
            let length = tail.iter().take_while(|byte| is_prerelease_byte(**byte)).count();
            if length == 0 {
                return None;
            }
            // The bytes were just checked to be ASCII, so this cannot fail.
            let text = std::str::from_utf8(&tail[..length]).ok()?;
            (Some(text.to_string()), &tail[length..])
        }
        None => (None, rest),
    };

    return Some((Version { major, minor, patch, prerelease }, rest));
}

fn take_number(bytes: &[u8]) -> Option<(u32, &[u8])> {
    let length = bytes.iter().take_while(|byte| byte.is_ascii_digit()).count();
    if length == 0 || length > MAX_DIGITS {
        return None;
    }
    let mut value = 0u32;
    for byte in &bytes[..length] {
        value = value * 10 + u32::from(byte - b'0');
    }
    return Some((value, &bytes[length..]));
}

fn is_prerelease_byte(byte: u8) -> bool {
    return byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.';
}

/// Length of the first line including its newline, or None if there is none.
fn line_end(bytes: &[u8]) -> Option<usize> {
    return bytes.iter().position(|byte| *byte == b'\n').map(|index| index + 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin_of(source: &str) -> Option<Pin> {
        return read_version_line(source.as_bytes());
    }

    #[test]
    fn reads_an_exact_pin() {
        assert_eq!(pin_of("nail 0.3.1\nprint(`hi`);\n"), Some(Pin::Exact(Version::new(0, 3, 1))));
    }

    #[test]
    fn reads_the_latest_sentinel() {
        assert_eq!(pin_of("nail latest\nprint(`hi`);\n"), Some(Pin::Latest));
    }

    #[test]
    fn a_file_with_no_version_line_is_unpinned() {
        assert_eq!(pin_of("print(`hi`);\n"), None);
        assert_eq!(strip_header("print(`hi`);\n"), ("print(`hi`);\n", 0));
    }

    #[test]
    fn a_shebang_may_sit_above_the_version_line() {
        let source = "#!/usr/bin/env nail\nnail 1.2.3\nprint(`hi`);\n";
        assert_eq!(pin_of(source), Some(Pin::Exact(Version::new(1, 2, 3))));
        assert_eq!(strip_header(source), ("print(`hi`);\n", 2));
    }

    #[test]
    fn a_shebang_alone_still_strips() {
        let source = "#!/usr/bin/env nail\nprint(`hi`);\n";
        assert_eq!(pin_of(source), None);
        assert_eq!(strip_header(source), ("print(`hi`);\n", 1));
    }

    #[test]
    fn a_byte_order_mark_is_skipped() {
        let source = "\u{FEFF}nail 0.1.0\nprint(`hi`);\n";
        assert_eq!(pin_of(source), Some(Pin::Exact(Version::new(0, 1, 0))));
        assert_eq!(strip_header(source), ("print(`hi`);\n", 1));
    }

    #[test]
    fn carriage_returns_are_tolerated() {
        assert_eq!(pin_of("nail 0.3.1\r\nprint(`hi`);\r\n"), Some(Pin::Exact(Version::new(0, 3, 1))));
    }

    #[test]
    fn a_version_line_may_be_the_whole_file() {
        assert_eq!(pin_of("nail 0.3.1"), Some(Pin::Exact(Version::new(0, 3, 1))));
        assert_eq!(strip_header("nail 0.3.1"), ("", 1));
    }

    #[test]
    fn prereleases_parse_and_are_marked() {
        let pin = pin_of("nail 0.4.0-dev\n").expect("a prerelease is a valid pin");
        match pin {
            Pin::Exact(version) => {
                assert_eq!(version.prerelease.as_deref(), Some("dev"));
                assert!(version.is_prerelease());
            }
            Pin::Latest => panic!("expected an exact pin"),
        }
    }

    #[test]
    fn ranges_cannot_be_written() {
        // The whole no-rot rule rests on this: there is no syntax for a set of
        // versions, so there is nothing to resolve and nothing to drift.
        assert_eq!(pin_of("nail ^0.3\n"), None);
        assert_eq!(pin_of("nail 0.3\n"), None);
        assert_eq!(pin_of("nail >=0.3.1\n"), None);
        assert_eq!(pin_of("nail 0.3.*\n"), None);
    }

    #[test]
    fn a_malformed_first_line_reads_as_unpinned_not_as_an_error() {
        // Might be syntax from a release that does not exist yet.
        assert_eq!(pin_of("nail 0.3.1 please\n"), None);
        assert_eq!(pin_of("nailish 0.3.1\n"), None);
        assert_eq!(pin_of("  nail 0.3.1\n"), None);
        assert_eq!(pin_of("nail  0.3.1\n"), None);
    }

    #[test]
    fn a_version_line_below_line_one_is_not_a_version_line() {
        assert_eq!(pin_of("print(`hi`);\nnail 0.3.1\n"), None);
    }

    #[test]
    fn digits_cannot_overflow() {
        assert_eq!(pin_of("nail 9999999999.0.0\n"), None);
        assert_eq!(pin_of("nail 999999999.0.0\n"), Some(Pin::Exact(Version::new(999999999, 0, 0))));
    }

    #[test]
    fn a_body_that_is_not_utf8_still_resolves() {
        let mut head = b"nail 0.3.1\n".to_vec();
        head.extend_from_slice(&[0xFF, 0xFE, 0x00]);
        assert_eq!(read_version_line(&head), Some(Pin::Exact(Version::new(0, 3, 1))));
    }

    #[test]
    fn versions_order_by_number_not_by_name() {
        let mut versions = vec![Version::new(0, 9, 0), Version::new(0, 10, 0), Version::new(0, 2, 1)];
        versions.sort();
        assert_eq!(versions, vec![Version::new(0, 2, 1), Version::new(0, 9, 0), Version::new(0, 10, 0)]);
    }

    #[test]
    fn a_prerelease_sorts_below_its_release() {
        let dev = Version { major: 0, minor: 4, patch: 0, prerelease: Some("dev".to_string()) };
        assert!(dev < Version::new(0, 4, 0));
        assert!(dev > Version::new(0, 3, 9));
    }

    #[test]
    fn versions_round_trip_through_text() {
        for text in ["0.0.1", "1.2.3", "0.4.0-dev", "10.20.30-rc.1"] {
            let version: Version = text.parse().expect("should parse");
            assert_eq!(version.to_string(), text);
        }
        assert!("0.3".parse::<Version>().is_err());
        assert!("0.3.1 ".parse::<Version>().is_err());
    }

    #[test]
    fn stamping_inserts_a_version_line() {
        assert_eq!(stamp("print(`hi`);\n", &Pin::Exact(Version::new(0, 3, 1))), "nail 0.3.1\nprint(`hi`);\n");
    }

    #[test]
    fn stamping_replaces_an_existing_version_line() {
        assert_eq!(stamp("nail 0.1.0\nprint(`hi`);\n", &Pin::Exact(Version::new(0, 3, 1))), "nail 0.3.1\nprint(`hi`);\n");
        assert_eq!(stamp("nail 0.1.0\nprint(`hi`);\n", &Pin::Latest), "nail latest\nprint(`hi`);\n");
    }

    #[test]
    fn stamping_keeps_a_shebang_on_top() {
        assert_eq!(stamp("#!/usr/bin/env nail\nprint(`hi`);\n", &Pin::Latest), "#!/usr/bin/env nail\nnail latest\nprint(`hi`);\n");
        assert_eq!(stamp("#!/usr/bin/env nail\nnail 0.1.0\nprint(`hi`);\n", &Pin::Exact(Version::new(2, 0, 0))), "#!/usr/bin/env nail\nnail 2.0.0\nprint(`hi`);\n");
    }

    #[test]
    fn stamping_is_idempotent() {
        let once = stamp("print(`hi`);\n", &Pin::Latest);
        assert_eq!(stamp(&once, &Pin::Latest), once);
    }
}
