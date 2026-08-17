//! Cutting a failing program down to the part that fails.
//!
//! A fuzzer's first version of a bug is a two hundred line program with four
//! unrelated mutations in it. Nobody can read that, and it cannot become a
//! regression test. Shrinking removes everything that is not needed to keep
//! the same failure, which usually leaves three or four lines that say
//! exactly what the bug is.

/// Cut `source` down while `still_fails` keeps answering true. The predicate
/// decides what "the same failure" means, because the caller knows whether it
/// is holding a panic in one place, a rejected build, or a process that died.
///
/// The passes are the classic delta debugging shape: try large cuts first,
/// then smaller ones, and start over whenever a cut lands. `budget` caps how
/// many times the predicate runs, because for a crash each call is a whole
/// process and a pathological case could otherwise shrink forever.
pub fn shrink(source: &str, budget: usize, still_fails: impl Fn(&str) -> bool) -> String {
    let mut best: Vec<String> = source.lines().map(String::from).collect();
    let mut spent = 0;

    // The version line is what the compiler reads first, and every real Nail
    // file has one, so it is never a candidate for removal.
    let keep_first = best.first().map_or(false, |line| line.starts_with("nail "));
    let floor = if keep_first { 1 } else { 0 };

    let mut chunk = (best.len() / 2).max(1);
    while chunk >= 1 && spent < budget {
        let mut index = floor;
        let mut cut_something = false;
        while index < best.len() && spent < budget {
            let end = (index + chunk).min(best.len());
            let mut candidate = best.clone();
            candidate.drain(index..end);
            spent += 1;
            if !candidate.is_empty() && still_fails(&joined(&candidate)) {
                best = candidate;
                cut_something = true;
            } else {
                index += chunk;
            }
        }
        if cut_something {
            // Something came out, so the large cuts are worth trying again
            // before moving on to finer ones.
            chunk = (best.len() / 2).max(1);
            if best.len() <= 2 {
                break;
            }
            continue;
        }
        if chunk == 1 {
            break;
        }
        chunk /= 2;
    }

    // A last pass over the surviving lines, trimming each one from the right.
    // A line that only needs its first half is common after a mutation that
    // grafted a fragment onto the end of one.
    let mut index = floor;
    while index < best.len() && spent < budget {
        let line = best[index].clone();
        let mut length = line.chars().count();
        while length > 1 && spent < budget {
            let shorter: String = line.chars().take(length / 2).collect();
            let mut candidate = best.clone();
            candidate[index] = shorter;
            spent += 1;
            if still_fails(&joined(&candidate)) {
                best = candidate;
                length /= 2;
            } else {
                break;
            }
        }
        index += 1;
    }

    joined(&best)
}

fn joined(lines: &[String]) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
}
