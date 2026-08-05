//! Runtime profiling for transpiled Nail programs.
//!
//! Profiling is on by default for every build (nailc --no-profile turns it
//! off, which is what the deploy script does). The transpiler emits three
//! things into a program: one `init` call at the top of main listing every
//! user function, one drop guard at the top of every user function body, and
//! one `finish` call at the end of main. A function's identity is a static
//! integer index into that name list, assigned at transpile time, so
//! recording a call is two clock reads and three relaxed atomic updates with
//! no locks and no hashing.
//!
//! While the program runs, a background task rewrites `.nail_profile.json`
//! every second so the IDE can annotate function declarations live. `finish`
//! writes a final snapshot and prints a timing sheet to stderr, but only when
//! stderr is a terminal, so piped output and test goldens never see it.
//!
//! A guard measures wall time from entry to exit. For an async function that
//! includes time spent awaiting and time suspended, which is what a caller
//! actually waits for. Times are cumulative: a caller's time includes its
//! callees, and recursive calls count each frame.

/// Stable fingerprint of a source file, embedded in the profile dump so the
/// IDE can tell whether timings still describe the code on screen. FNV-1a by
/// hand because std's DefaultHasher is not guaranteed stable across builds,
/// and the compiler and the IDE must agree byte for byte.
pub fn source_fingerprint(source: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

#[cfg(not(target_arch = "wasm32"))]
mod real {
    use std::io::IsTerminal;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::OnceLock;
    use std::time::Instant;

    pub struct FnStat {
        total_nanos: AtomicU64,
        calls: AtomicU64,
        max_nanos: AtomicU64,
    }

    struct Profile {
        source_hash: &'static str,
        names: &'static [&'static str],
        stats: Vec<FnStat>,
        start: Instant,
    }

    static PROFILE: OnceLock<Profile> = OnceLock::new();

    // A flat file, not .nail/profile.json: .nail is already the IDE's
    // settings file, so a directory of that name can never be created.
    const DUMP_PATH: &str = ".nail_profile.json";
    const DUMP_TMP_PATH: &str = ".nail_profile.json.tmp";

    /// Called once at the top of a transpiled main. `names` holds every user
    /// function in source order, and each function's guard id is its index.
    /// `source_hash` fingerprints the source the program was built from.
    pub fn init(source_hash: &'static str, names: &'static [&'static str]) {
        let stats = names.iter().map(|_| FnStat { total_nanos: AtomicU64::new(0), calls: AtomicU64::new(0), max_nanos: AtomicU64::new(0) }).collect();
        if PROFILE.set(Profile { source_hash, names, stats, start: Instant::now() }).is_err() {
            return;
        }
        // Keeps .nail/profile.json fresh for programs that never reach finish,
        // like servers. The task dies with the runtime when main returns.
        tokio::spawn(async {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                dump();
            }
        });
    }

    pub struct Guard {
        id: usize,
        start: Instant,
    }

    /// Emitted as the first statement of every user function body. Recording
    /// rides the drop so early returns are measured too.
    #[inline]
    pub fn guard(id: usize) -> Guard {
        Guard { id, start: Instant::now() }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            if let Some(profile) = PROFILE.get() {
                let nanos = self.start.elapsed().as_nanos() as u64;
                let stat = &profile.stats[self.id];
                stat.total_nanos.fetch_add(nanos, Ordering::Relaxed);
                stat.calls.fetch_add(1, Ordering::Relaxed);
                stat.max_nanos.fetch_max(nanos, Ordering::Relaxed);
            }
        }
    }

    /// Called at the end of a transpiled main: final snapshot to disk, and a
    /// timing sheet on stderr when a human is watching. One snapshot feeds
    /// both so the file and the sheet agree.
    pub fn finish() {
        let Some(snapshot) = snapshot() else { return };
        write_dump(&snapshot);
        if std::io::stderr().is_terminal() {
            eprint!("{}", render_sheet(&snapshot));
        }
    }

    struct Row {
        name: &'static str,
        calls: u64,
        total_nanos: u64,
        max_nanos: u64,
    }

    struct Snapshot {
        source_hash: &'static str,
        wall_nanos: u64,
        rows: Vec<Row>,
    }

    fn snapshot() -> Option<Snapshot> {
        let profile = PROFILE.get()?;
        let rows = profile
            .names
            .iter()
            .zip(profile.stats.iter())
            .map(|(name, stat)| Row {
                name,
                calls: stat.calls.load(Ordering::Relaxed),
                total_nanos: stat.total_nanos.load(Ordering::Relaxed),
                max_nanos: stat.max_nanos.load(Ordering::Relaxed),
            })
            .collect();
        Some(Snapshot { source_hash: profile.source_hash, wall_nanos: profile.start.elapsed().as_nanos() as u64, rows })
    }

    fn dump() {
        let Some(snapshot) = snapshot() else { return };
        write_dump(&snapshot);
    }

    fn write_dump(snapshot: &Snapshot) {
        // Temp file plus rename so a watcher never reads a half-written file
        if std::fs::write(DUMP_TMP_PATH, render_json(snapshot)).is_ok() {
            let _ = std::fs::rename(DUMP_TMP_PATH, DUMP_PATH);
        }
    }

    fn render_json(snapshot: &Snapshot) -> String {
        let functions: Vec<serde_json::Value> = snapshot
            .rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "name": row.name,
                    "calls": row.calls,
                    "total_nanos": row.total_nanos,
                    "max_nanos": row.max_nanos,
                })
            })
            .collect();
        serde_json::json!({ "source_hash": snapshot.source_hash, "wall_nanos": snapshot.wall_nanos, "functions": functions }).to_string()
    }

    fn format_duration(nanos: u64) -> String {
        if nanos < 1_000 {
            format!("{}ns", nanos)
        } else if nanos < 1_000_000 {
            format!("{:.1}µs", nanos as f64 / 1_000.0)
        } else if nanos < 1_000_000_000 {
            format!("{:.1}ms", nanos as f64 / 1_000_000.0)
        } else {
            format!("{:.2}s", nanos as f64 / 1_000_000_000.0)
        }
    }

    const SHEET_MAX_ROWS: usize = 24;

    fn render_sheet(snapshot: &Snapshot) -> String {
        let mut rows: Vec<&Row> = snapshot.rows.iter().filter(|row| row.calls > 0).collect();
        if rows.is_empty() {
            return String::new();
        }
        rows.sort_by(|a, b| b.total_nanos.cmp(&a.total_nanos));
        let hidden = rows.len().saturating_sub(SHEET_MAX_ROWS);
        rows.truncate(SHEET_MAX_ROWS);

        let name_width = rows.iter().map(|row| row.name.len()).max().unwrap_or(0).max("function".len());
        let mut out = String::new();
        out.push_str("\n── nail timing sheet ──\n");
        out.push_str(&format!("{:<name_width$}  {:>8}  {:>9}  {:>9}  {:>9}  {:>6}\n", "function", "calls", "total", "avg", "max", "%"));
        for row in &rows {
            let percent = if snapshot.wall_nanos > 0 { row.total_nanos as f64 / snapshot.wall_nanos as f64 * 100.0 } else { 0.0 };
            out.push_str(&format!(
                "{:<name_width$}  {:>8}  {:>9}  {:>9}  {:>9}  {:>5.1}%\n",
                row.name,
                row.calls,
                format_duration(row.total_nanos),
                format_duration(row.total_nanos / row.calls),
                format_duration(row.max_nanos),
                percent
            ));
        }
        if hidden > 0 {
            out.push_str(&format!("({} more not shown)\n", hidden));
        }
        let parallel_note = if rows.iter().any(|row| snapshot.wall_nanos > 0 && row.total_nanos > snapshot.wall_nanos) {
            " A share over 100% means that function ran on several cores at once."
        } else {
            ""
        };
        out.push_str(&format!(
            "program wall time {}, it could run {} times a second. Times are cumulative, a caller includes its callees.{}\n",
            format_duration(snapshot.wall_nanos),
            format_runs_per_second(snapshot.wall_nanos),
            parallel_note
        ));
        out
    }

    /// The whole program's speed as one number a person can judge: how many
    /// times per second this entire run could repeat. Over 60 reads as
    /// instant, single digits read as sluggish, under 1 is a long job.
    fn format_runs_per_second(wall_nanos: u64) -> String {
        let per_second = 1_000_000_000.0 / wall_nanos.max(1) as f64;
        if per_second >= 1_000_000.0 {
            format!("{:.1} million", per_second / 1_000_000.0)
        } else if per_second >= 1_000.0 {
            format!("{:.1} thousand", per_second / 1_000.0)
        } else if per_second >= 100.0 {
            format!("{:.0}", per_second)
        } else if per_second >= 1.0 {
            format!("{:.1}", per_second)
        } else {
            format!("{:.2}", per_second)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn sample_snapshot() -> Snapshot {
            Snapshot {
                source_hash: "00000000deadbeef",
                wall_nanos: 2_000_000_000,
                rows: vec![
                    Row { name: "handle_request", calls: 312, total_nanos: 1_020_000_000, max_nanos: 41_000_000 },
                    Row { name: "parse_row", calls: 14800, total_nanos: 402_000_000, max_nanos: 1_100_000 },
                    Row { name: "never_called", calls: 0, total_nanos: 0, max_nanos: 0 },
                ],
            }
        }

        #[test]
        fn test_format_duration_picks_sane_units() {
            assert_eq!(format_duration(950), "950ns");
            assert_eq!(format_duration(27_100), "27.1µs");
            assert_eq!(format_duration(3_300_000), "3.3ms");
            assert_eq!(format_duration(1_230_000_000), "1.23s");
        }


        #[test]
        fn test_sheet_sorts_by_total_and_skips_uncalled() {
            let sheet = render_sheet(&sample_snapshot());
            let handle_pos = sheet.find("handle_request").expect("busiest function shown");
            let parse_pos = sheet.find("parse_row").expect("second function shown");
            assert!(handle_pos < parse_pos, "sheet is sorted by total time descending");
            assert!(!sheet.contains("never_called"), "uncalled functions stay off the sheet");
            assert!(sheet.contains("51.0%"), "percent is share of wall time");
            assert!(sheet.contains("it could run 0.50 times a second"), "the footer carries whole program runs per second");
        }

        #[test]
        fn test_runs_per_second_reads_like_a_verdict() {
            assert_eq!(format_runs_per_second(16_100_000), "62.1");
            assert_eq!(format_runs_per_second(100_000_000), "10.0");
            assert_eq!(format_runs_per_second(9_660_000_000), "0.10");
            assert_eq!(format_runs_per_second(80_000), "12.5 thousand");
        }

        #[test]
        fn test_sheet_empty_when_nothing_ran() {
            let snapshot = Snapshot { source_hash: "", wall_nanos: 5, rows: vec![Row { name: "lonely", calls: 0, total_nanos: 0, max_nanos: 0 }] };
            assert_eq!(render_sheet(&snapshot), "");
        }

        #[test]
        fn test_json_shape_matches_ide_contract() {
            let json = render_json(&sample_snapshot());
            let parsed: serde_json::Value = serde_json::from_str(&json).expect("dump is valid json");
            assert_eq!(parsed["source_hash"], "00000000deadbeef");
            assert_eq!(parsed["wall_nanos"], 2_000_000_000u64);
            assert_eq!(parsed["functions"][0]["name"], "handle_request");
            assert_eq!(parsed["functions"][0]["calls"], 312);
            assert_eq!(parsed["functions"][0]["total_nanos"], 1_020_000_000u64);
            assert_eq!(parsed["functions"][0]["max_nanos"], 41_000_000u64);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use real::{finish, guard, init, Guard};

// The browser build has no monotonic clock, no filesystem and no terminal, so
// profiling compiles to nothing there and instrumented programs still build.
#[cfg(target_arch = "wasm32")]
mod stub {
    pub struct Guard;

    pub fn init(_source_hash: &'static str, _names: &'static [&'static str]) {}

    #[inline]
    pub fn guard(_id: usize) -> Guard {
        Guard
    }

    pub fn finish() {}
}

#[cfg(target_arch = "wasm32")]
pub use stub::{finish, guard, init, Guard};

#[cfg(test)]
mod fingerprint_tests {
    use super::source_fingerprint;

    #[test]
    fn test_fingerprint_is_stable_fnv1a() {
        // Pinned reference values. The IDE compares these against dumps from
        // separately built programs, so the algorithm can never drift.
        assert_eq!(source_fingerprint(""), "cbf29ce484222325");
        assert_eq!(source_fingerprint("a"), "af63dc4c8601ec8c");
        assert_ne!(source_fingerprint("f a():i { r 1; }"), source_fingerprint("f a():i { r 2; }"));
    }
}
