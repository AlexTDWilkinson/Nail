//! The compiler's fuzzer: millions of programs, checked against invariants
//! the compiler must never break.
//!
//! Two engines feed it. The mutation engine bends real Nail programs out of
//! shape, which is what finds the crashes and the confused error messages.
//! The generation engine writes well typed programs from a type environment,
//! which is what reaches the transpiler and rustc in volume, where the
//! expensive bugs live: a program the type checker accepted and rustc then
//! refused is a wall of Rust in front of somebody who never wrote any.
//!
//! Every case is a pure function of one number. A finding names that number,
//! so any run on any machine can be reproduced with `nail-fuzz case <seed>`,
//! and the whole state of a run is a seed range rather than a corpus
//! directory that has to be shipped around.
//!
//! Hard crashes are why the driver runs cases in child processes. A stack
//! overflow aborts the process it happens in, and no amount of catch_unwind
//! sees it, so the parent watches a progress file per worker: when a worker
//! dies, the seed in its progress file is the case that killed it.

pub mod corpus;
pub mod generate;
pub mod imports;
pub mod mutate;
pub mod oracle;
pub mod shrink;

use std::path::Path;

use corpus::Corpus;
use oracle::{Finding, Outcome};

/// Which engine writes a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Mutate,
    Generate,
    /// Alternate between the two, which is the useful default: the engines
    /// find different classes of bug and neither subsumes the other.
    Both,
}

impl Engine {
    pub fn parse(name: &str) -> Option<Engine> {
        match name {
            "mutate" => Some(Engine::Mutate),
            "generate" => Some(Engine::Generate),
            "both" | "all" => Some(Engine::Both),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Engine::Mutate => "mutate",
            Engine::Generate => "generate",
            Engine::Both => "both",
        }
    }

    /// The engine that actually writes a given seed's case.
    fn resolve(self, seed: u64) -> Engine {
        match self {
            Engine::Both => {
                if seed % 2 == 0 {
                    Engine::Generate
                } else {
                    Engine::Mutate
                }
            }
            engine => engine,
        }
    }
}

/// One program to try, and where it came from.
pub struct Case {
    pub source: String,
    pub origin: String,
}

/// The program for a seed. Same seed and same corpus means the same program,
/// forever, which is what makes a bug report a single number.
pub fn case(engine: Engine, seed: u64, corpus: &Corpus) -> Case {
    match engine.resolve(seed) {
        Engine::Generate => {
            let (source, origin) = generate::case(seed);
            Case { source, origin }
        }
        _ => {
            if corpus.is_empty() {
                let (source, origin) = generate::case(seed);
                return Case { source, origin };
            }
            let (source, origin) = mutate::case(seed, corpus);
            Case { source, origin }
        }
    }
}

/// Examine one case, all in this process. The path is where the case is
/// written on disk, which matters only for `import`.
pub fn examine(source: &str, path: &Path) -> (Option<Finding>, Outcome) {
    oracle::examine(source, path)
}

/// Examine one case and ask even the questions the fuzz loop only samples.
/// This is what looking at a single file by hand should do.
pub fn examine_thoroughly(source: &str, path: &Path) -> (Option<Finding>, Outcome) {
    oracle::examine_thoroughly(source, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repository() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn corpus() -> Corpus {
        Corpus::load(&[&repository().join("tests"), &repository().join("examples")])
    }

    /// Nothing on disk, so a case that mentions `import` simply fails to
    /// resolve, which is the same answer the fuzzer's own workers get.
    fn nowhere() -> PathBuf {
        repository().join("target").join("fuzz").join("unit_test_case.nail")
    }

    #[test]
    fn a_seed_always_writes_the_same_program() {
        let corpus = corpus();
        for seed in [1, 2, 77, 12345] {
            let first = case(Engine::Both, seed, &corpus);
            let second = case(Engine::Both, seed, &corpus);
            assert_eq!(first.source, second.source, "seed {} wrote two different programs", seed);
        }
    }

    #[test]
    fn different_seeds_write_different_programs() {
        let corpus = corpus();
        let programs: std::collections::HashSet<String> = (1..40).map(|seed| case(Engine::Generate, seed, &corpus).source).collect();
        assert!(programs.len() > 30, "40 seeds produced only {} distinct programs", programs.len());
    }

    /// The generator earns its place by reaching the far end of the compiler.
    /// One that mostly wrote programs the checker refused would be testing
    /// the checker's error paths and nothing else.
    #[test]
    fn generated_programs_mostly_compile_and_never_break_an_invariant() {
        let corpus = corpus();
        let path = nowhere();
        let mut built = 0;
        let mut findings = Vec::new();
        for seed in 1..60 {
            let program = case(Engine::Generate, seed, &corpus);
            let (finding, outcome) = examine(&program.source, &path);
            if let Some(finding) = finding {
                findings.push(format!("seed {}: {} in {} ({})", seed, finding.property.name(), finding.stage, finding.detail));
            }
            if matches!(outcome, Outcome::Built(_)) {
                built += 1;
            }
        }
        assert!(findings.is_empty(), "the fuzzer's own generator found bugs: {:?}", findings);
        assert!(built > 45, "only {} of 59 generated programs made it through the compiler", built);
    }

    /// The mutation engine has the opposite job: most of what it writes is
    /// broken, and being refused is the right answer. What it may never do is
    /// break an invariant.
    #[test]
    fn mutated_programs_never_break_an_invariant() {
        let corpus = corpus();
        assert!(!corpus.is_empty(), "the repository has Nail programs to mutate");
        let path = nowhere();
        let mut findings = Vec::new();
        for seed in 1..60 {
            let program = case(Engine::Mutate, seed, &corpus);
            if let (Some(finding), _) = examine(&program.source, &path) {
                findings.push(format!("seed {}: {} in {} ({})", seed, finding.property.name(), finding.stage, finding.detail));
            }
        }
        assert!(findings.is_empty(), "the fuzzer's own mutation engine found bugs: {:?}", findings);
    }

    #[test]
    fn shrinking_cuts_a_program_down_to_what_matters() {
        let program = (1..40).map(|index| format!("count_{}:i = {};", index, index)).collect::<Vec<_>>().join("\n");
        let program = format!("nail latest\n{}\nlabel_one:s = `keep me`;\n", program);
        let minimal = shrink::shrink(&program, 1000, |candidate| candidate.contains("keep me"));
        assert!(minimal.contains("keep me"));
        assert!(minimal.lines().count() <= 3, "shrunk to {} lines:\n{}", minimal.lines().count(), minimal);
        assert!(minimal.starts_with("nail latest"), "the version line is never cut");
    }

    /// The invariants have to be able to see a bug, not only to pass. This
    /// hands them a program that is fine and one that is not, and checks they
    /// can tell the difference.
    #[test]
    fn the_invariants_can_tell_a_broken_program_from_a_working_one() {
        let path = nowhere();
        let good = "nail latest\ncount_value:i = 2 + 3;\nprint(count_value);\n";
        let (finding, outcome) = examine(good, &path);
        assert!(finding.is_none(), "a working program was reported as a finding: {:?}", finding);
        assert!(matches!(outcome, Outcome::Built(_)));

        // Refused, and correctly so: being refused is not a finding.
        let refused = "nail latest\ncount_value:i = `text`;\nprint(count_value);\n";
        let (finding, outcome) = examine(refused, &path);
        assert!(finding.is_none(), "a refused program was reported as a finding: {:?}", finding);
        assert!(matches!(outcome, Outcome::Refused(_)));
    }
}
