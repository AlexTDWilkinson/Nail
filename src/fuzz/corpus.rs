//! The programs the mutation engine starts from.
//!
//! Random bytes almost never reach the type checker, so the mutator works
//! from real Nail instead: every `.nail` file in the repository, bent out of
//! shape a few edits at a time. A file that already type checks is a case
//! that only has to survive one wrong turn to land somewhere interesting.

use std::fs;
use std::path::{Path, PathBuf};

pub struct Entry {
    pub path: PathBuf,
    pub source: String,
    /// The file split into lines once, because every mutation works in lines
    /// and splitting per case would cost more than the mutation does.
    pub lines: Vec<String>,
}

pub struct Corpus {
    pub entries: Vec<Entry>,
}

impl Corpus {
    /// Load every `.nail` file under the given roots, sorted by path so the
    /// same seed means the same case on any machine with the same tree.
    pub fn load(roots: &[&Path]) -> Corpus {
        let mut paths = Vec::new();
        for root in roots {
            collect(root, &mut paths);
        }
        paths.sort();
        let entries = paths
            .into_iter()
            .filter_map(|path| {
                let source = fs::read_to_string(&path).ok()?;
                // Enormous files are poor seeds: every mutation of one costs
                // in proportion to its length, and the bugs live in the
                // shapes rather than in the size.
                if source.is_empty() || source.len() > 60_000 {
                    return None;
                }
                let lines = source.lines().map(String::from).collect();
                Some(Entry { path, source, lines })
            })
            .collect();
        Corpus { entries }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn pick(&self, index: u64) -> &Entry {
        &self.entries[(index % self.entries.len() as u64) as usize]
    }

    /// A number that changes when the corpus changes. A seed only reproduces
    /// a case against the corpus it was drawn from, so a run reports this and
    /// a reproduction checks it.
    pub fn fingerprint(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for entry in &self.entries {
            for byte in entry.path.to_string_lossy().as_bytes().iter().chain(entry.source.as_bytes()) {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        hash
    }
}

fn collect(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().map_or(false, |extension| extension == "nail") {
            out.push(path);
        }
    }
}
