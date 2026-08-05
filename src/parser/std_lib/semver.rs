//! Version numbers, compared the way tools compare them.
//!
//! Nail pins the exact compiler version a file was written for, so programs
//! that build or deploy other programs end up asking whether one version is
//! newer than another. Comparing the text does not answer that - `1.10.0` is
//! newer than `1.9.0` but sorts before it - so the comparison has to know the
//! parts are numbers.
//!
//! The subset handled here is the one real version strings use: three numbers,
//! an optional `-prerelease` and an optional `+build`. Comparison follows the
//! semver rules: numbers first, then a prerelease losing to the release it
//! precedes, and build metadata ignored entirely.

/// One version, already split into the parts that get compared.
struct Version {
    major: i64,
    minor: i64,
    patch: i64,
    prerelease: String,
}

/// Splits a version string into its parts, naming what was wrong with it when
/// it will not split. A leading `v`, as tags are usually written, is allowed.
fn parse_version(text: &str) -> Result<Version, String> {
    let trimmed = text.trim();
    let without_prefix = trimmed.strip_prefix('v').unwrap_or(trimmed);

    // Build metadata takes no part in comparison, so it is dropped as soon as
    // it is found rather than carried around unused.
    let without_build = match without_prefix.split_once('+') {
        Some((before, _)) => before,
        None => without_prefix,
    };
    let (numbers, prerelease) = match without_build.split_once('-') {
        Some((before, after)) => (before, after.to_string()),
        None => (without_build, String::new()),
    };

    let parts: Vec<&str> = numbers.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("`{}` is not a version: it needs exactly three numbers, as in 1.4.2", text));
    }
    let mut values = [0i64; 3];
    for (position, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.chars().all(|character| character.is_ascii_digit()) {
            return Err(format!("`{}` is not a version: `{}` is not a number", text, part));
        }
        values[position] = part.parse::<i64>().map_err(|_| format!("`{}` is not a version: `{}` is too large a number", text, part))?;
    }

    return Ok(Version { major: values[0], minor: values[1], patch: values[2], prerelease });
}

/// Compares two prerelease strings by the semver rule: each dot-separated
/// identifier is compared as a number when both are numeric and as text
/// otherwise, and a version that runs out of identifiers first is the lower.
fn compare_prereleases(left: &str, right: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // No prerelease at all outranks any prerelease: 1.0.0 is newer than
    // 1.0.0-rc.1, which is the whole point of the suffix.
    match (left.is_empty(), right.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => {}
    }

    let mut left_parts = left.split('.');
    let mut right_parts = right.split('.');
    loop {
        match (left_parts.next(), right_parts.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_part), Some(right_part)) => {
                let ordering = match (left_part.parse::<i64>(), right_part.parse::<i64>()) {
                    (Ok(left_number), Ok(right_number)) => left_number.cmp(&right_number),
                    // A numeric identifier always ranks below a text one.
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => left_part.cmp(right_part),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

fn compare_versions(left: &Version, right: &Version) -> std::cmp::Ordering {
    return left
        .major
        .cmp(&right.major)
        .then(left.minor.cmp(&right.minor))
        .then(left.patch.cmp(&right.patch))
        .then(compare_prereleases(&left.prerelease, &right.prerelease));
}

/// Whether the text is a version this module can read.
pub fn valid(version: &String) -> bool {
    return parse_version(version).is_ok();
}

/// -1 when the first version is older, 0 when they are the same version, and 1
/// when the first is newer. Build metadata is ignored, so `1.0.0+monday` and
/// `1.0.0+tuesday` are the same version.
pub fn compare(first: String, second: String) -> Result<i64, String> {
    let left = parse_version(&first).map_err(|detail| format!("semver_compare: {}", detail))?;
    let right = parse_version(&second).map_err(|detail| format!("semver_compare: {}", detail))?;
    return Ok(match compare_versions(&left, &right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    });
}

/// Whether the first version is newer than the second.
pub fn is_newer(first: String, second: String) -> Result<bool, String> {
    return Ok(compare(first, second)? > 0);
}

/// Whether the first version is older than the second.
pub fn is_older(first: String, second: String) -> Result<bool, String> {
    return Ok(compare(first, second)? < 0);
}

pub fn major(version: String) -> Result<i64, String> {
    return Ok(parse_version(&version).map_err(|detail| format!("semver_major: {}", detail))?.major);
}

pub fn minor(version: String) -> Result<i64, String> {
    return Ok(parse_version(&version).map_err(|detail| format!("semver_minor: {}", detail))?.minor);
}

pub fn patch(version: String) -> Result<i64, String> {
    return Ok(parse_version(&version).map_err(|detail| format!("semver_patch: {}", detail))?.patch);
}

/// The prerelease part without its hyphen, or the empty string when there is
/// none: `1.0.0-rc.1` gives `rc.1`.
pub fn prerelease(version: String) -> Result<String, String> {
    return Ok(parse_version(&version).map_err(|detail| format!("semver_prerelease: {}", detail))?.prerelease);
}

/// The next version after this one at the given level. Bumping a level resets
/// the ones below it and drops any prerelease, because 2.0.0 is what follows
/// 1.9.3 and 2.0.0-rc.1 is not a thing anyone means by "next major".
fn bump(version: String, level: &str) -> Result<String, String> {
    let parsed = parse_version(&version).map_err(|detail| format!("semver_bump_{}: {}", level, detail))?;
    return Ok(match level {
        "major" => format!("{}.0.0", parsed.major + 1),
        "minor" => format!("{}.{}.0", parsed.major, parsed.minor + 1),
        _ => format!("{}.{}.{}", parsed.major, parsed.minor, parsed.patch + 1),
    });
}

pub fn bump_major(version: String) -> Result<String, String> {
    return bump(version, "major");
}

pub fn bump_minor(version: String) -> Result<String, String> {
    return bump(version, "minor");
}

pub fn bump_patch(version: String) -> Result<String, String> {
    return bump(version, "patch");
}

/// Whether a version meets a requirement. The forms understood are the ones
/// written by hand: an exact version, a comparison (`>=1.2.0`, `<2.0.0`), a
/// caret (`^1.2.3`, meaning anything that keeps the leftmost non-zero number),
/// a tilde (`~1.2.3`, meaning anything up to the next minor), `*` for anything,
/// and several of these separated by commas, all of which must hold.
pub fn satisfies(version: String, requirement: String) -> Result<bool, String> {
    let parsed = parse_version(&version).map_err(|detail| format!("semver_satisfies: {}", detail))?;
    for clause in requirement.split(',') {
        if !clause_holds(&parsed, clause.trim())? {
            return Ok(false);
        }
    }
    return Ok(true);
}

fn clause_holds(version: &Version, clause: &str) -> Result<bool, String> {
    use std::cmp::Ordering;
    if clause.is_empty() || clause == "*" {
        return Ok(true);
    }

    let describe = |detail: String| format!("semver_satisfies: {}", detail);
    if let Some(rest) = clause.strip_prefix(">=") {
        let bound = parse_version(rest).map_err(describe)?;
        return Ok(compare_versions(version, &bound) != Ordering::Less);
    }
    if let Some(rest) = clause.strip_prefix("<=") {
        let bound = parse_version(rest).map_err(describe)?;
        return Ok(compare_versions(version, &bound) != Ordering::Greater);
    }
    if let Some(rest) = clause.strip_prefix('>') {
        let bound = parse_version(rest).map_err(describe)?;
        return Ok(compare_versions(version, &bound) == Ordering::Greater);
    }
    if let Some(rest) = clause.strip_prefix('<') {
        let bound = parse_version(rest).map_err(describe)?;
        return Ok(compare_versions(version, &bound) == Ordering::Less);
    }
    if let Some(rest) = clause.strip_prefix('^') {
        let bound = parse_version(rest).map_err(describe)?;
        if compare_versions(version, &bound) == Ordering::Less {
            return Ok(false);
        }
        // The caret allows changes that cannot break a caller, and before 1.0.0
        // that means the minor number is doing the major number's job.
        let ceiling = if bound.major > 0 {
            Version { major: bound.major + 1, minor: 0, patch: 0, prerelease: String::new() }
        } else if bound.minor > 0 {
            Version { major: 0, minor: bound.minor + 1, patch: 0, prerelease: String::new() }
        } else {
            Version { major: 0, minor: 0, patch: bound.patch + 1, prerelease: String::new() }
        };
        return Ok(compare_versions(version, &ceiling) == Ordering::Less);
    }
    if let Some(rest) = clause.strip_prefix('~') {
        let bound = parse_version(rest).map_err(describe)?;
        if compare_versions(version, &bound) == Ordering::Less {
            return Ok(false);
        }
        let ceiling = Version { major: bound.major, minor: bound.minor + 1, patch: 0, prerelease: String::new() };
        return Ok(compare_versions(version, &ceiling) == Ordering::Less);
    }

    let exact = parse_version(clause.strip_prefix('=').unwrap_or(clause)).map_err(describe)?;
    return Ok(compare_versions(version, &exact) == Ordering::Equal);
}

/// The versions sorted oldest first. Anything that is not a version is an
/// error naming it, rather than being quietly sorted as text.
pub fn sort(versions: Vec<String>) -> Result<Vec<String>, String> {
    let mut parsed: Vec<(Version, String)> = Vec::with_capacity(versions.len());
    for version in versions {
        let one = parse_version(&version).map_err(|detail| format!("semver_sort: {}", detail))?;
        parsed.push((one, version));
    }
    parsed.sort_by(|left, right| compare_versions(&left.0, &right.0));
    return Ok(parsed.into_iter().map(|(_, text)| text).collect());
}

/// The newest of the versions, which is what picking a release out of a list of
/// tags comes down to. An empty list is an error, because there is no answer.
pub fn newest(versions: Vec<String>) -> Result<String, String> {
    let sorted = sort(versions).map_err(|detail| detail.replace("semver_sort", "semver_newest"))?;
    return match sorted.last() {
        Some(newest) => Ok(newest.clone()),
        None => Err("semver_newest: there were no versions to choose from".to_string()),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_recognised_by_its_shape() {
        assert!(valid(&"1.4.2".to_string()));
        assert!(valid(&"v1.4.2".to_string()));
        assert!(valid(&"1.0.0-rc.1".to_string()));
        assert!(valid(&"1.0.0+build.5".to_string()));
        assert!(!valid(&"1.4".to_string()));
        assert!(!valid(&"1.4.x".to_string()));
        assert!(!valid(&"not a version".to_string()));
    }

    #[test]
    fn numbers_are_compared_as_numbers_not_as_text() {
        assert_eq!(compare("1.10.0".to_string(), "1.9.0".to_string()).expect("both valid"), 1);
        assert_eq!(compare("1.9.0".to_string(), "1.10.0".to_string()).expect("both valid"), -1);
        assert_eq!(compare("2.0.0".to_string(), "2.0.0".to_string()).expect("both valid"), 0);
    }

    #[test]
    fn build_metadata_does_not_change_which_version_it_is() {
        assert_eq!(compare("1.0.0+monday".to_string(), "1.0.0+tuesday".to_string()).expect("both valid"), 0);
    }

    #[test]
    fn a_prerelease_comes_before_the_release_it_precedes() {
        assert_eq!(compare("1.0.0-rc.1".to_string(), "1.0.0".to_string()).expect("both valid"), -1);
        assert_eq!(compare("1.0.0-rc.2".to_string(), "1.0.0-rc.10".to_string()).expect("both valid"), -1);
        assert_eq!(compare("1.0.0-alpha".to_string(), "1.0.0-beta".to_string()).expect("both valid"), -1);
        assert_eq!(compare("1.0.0-1".to_string(), "1.0.0-alpha".to_string()).expect("both valid"), -1);
    }

    #[test]
    fn newer_and_older_read_the_comparison_out_loud() {
        assert!(is_newer("2.0.0".to_string(), "1.9.9".to_string()).expect("both valid"));
        assert!(is_older("1.9.9".to_string(), "2.0.0".to_string()).expect("both valid"));
        assert!(!is_newer("1.0.0".to_string(), "1.0.0".to_string()).expect("both valid"));
    }

    #[test]
    fn the_parts_can_be_read_out() {
        assert_eq!(major("1.4.2".to_string()).expect("valid"), 1);
        assert_eq!(minor("1.4.2".to_string()).expect("valid"), 4);
        assert_eq!(patch("1.4.2".to_string()).expect("valid"), 2);
        assert_eq!(prerelease("1.0.0-rc.1".to_string()).expect("valid"), "rc.1");
        assert_eq!(prerelease("1.0.0".to_string()).expect("valid"), "");
    }

    #[test]
    fn bumping_a_level_resets_the_ones_below_it() {
        assert_eq!(bump_major("1.9.3".to_string()).expect("valid"), "2.0.0");
        assert_eq!(bump_minor("1.9.3".to_string()).expect("valid"), "1.10.0");
        assert_eq!(bump_patch("1.9.3".to_string()).expect("valid"), "1.9.4");
        assert_eq!(bump_patch("1.0.0-rc.1".to_string()).expect("valid"), "1.0.1");
    }

    #[test]
    fn an_exact_requirement_wants_that_version() {
        assert!(satisfies("1.4.2".to_string(), "1.4.2".to_string()).expect("both valid"));
        assert!(satisfies("1.4.2".to_string(), "=1.4.2".to_string()).expect("both valid"));
        assert!(!satisfies("1.4.3".to_string(), "1.4.2".to_string()).expect("both valid"));
    }

    #[test]
    fn comparisons_bound_the_version_from_one_side() {
        assert!(satisfies("1.5.0".to_string(), ">=1.2.0".to_string()).expect("both valid"));
        assert!(satisfies("1.2.0".to_string(), ">=1.2.0".to_string()).expect("both valid"));
        assert!(!satisfies("1.1.0".to_string(), ">=1.2.0".to_string()).expect("both valid"));
        assert!(satisfies("1.9.9".to_string(), "<2.0.0".to_string()).expect("both valid"));
        assert!(!satisfies("2.0.0".to_string(), "<2.0.0".to_string()).expect("both valid"));
        assert!(satisfies("2.0.0".to_string(), "<=2.0.0".to_string()).expect("both valid"));
        assert!(satisfies("2.0.1".to_string(), ">2.0.0".to_string()).expect("both valid"));
    }

    #[test]
    fn several_clauses_must_all_hold() {
        assert!(satisfies("1.5.0".to_string(), ">=1.2.0, <2.0.0".to_string()).expect("both valid"));
        assert!(!satisfies("2.1.0".to_string(), ">=1.2.0, <2.0.0".to_string()).expect("both valid"));
    }

    #[test]
    fn a_caret_keeps_the_leftmost_non_zero_number() {
        assert!(satisfies("1.2.3".to_string(), "^1.2.3".to_string()).expect("both valid"));
        assert!(satisfies("1.9.0".to_string(), "^1.2.3".to_string()).expect("both valid"));
        assert!(!satisfies("2.0.0".to_string(), "^1.2.3".to_string()).expect("both valid"));
        assert!(!satisfies("1.2.2".to_string(), "^1.2.3".to_string()).expect("both valid"));
        // Before 1.0.0 the minor number is what a caret protects.
        assert!(satisfies("0.2.9".to_string(), "^0.2.3".to_string()).expect("both valid"));
        assert!(!satisfies("0.3.0".to_string(), "^0.2.3".to_string()).expect("both valid"));
        assert!(!satisfies("0.0.4".to_string(), "^0.0.3".to_string()).expect("both valid"));
    }

    #[test]
    fn a_tilde_allows_only_the_last_number_to_move() {
        assert!(satisfies("1.2.9".to_string(), "~1.2.3".to_string()).expect("both valid"));
        assert!(!satisfies("1.3.0".to_string(), "~1.2.3".to_string()).expect("both valid"));
        assert!(!satisfies("1.2.2".to_string(), "~1.2.3".to_string()).expect("both valid"));
    }

    #[test]
    fn a_star_takes_anything() {
        assert!(satisfies("9.9.9".to_string(), "*".to_string()).expect("both valid"));
    }

    #[test]
    fn sorting_puts_the_oldest_first() {
        let sorted = sort(vec!["1.10.0".to_string(), "1.9.0".to_string(), "2.0.0-rc.1".to_string(), "2.0.0".to_string()]).expect("all valid");
        assert_eq!(sorted, vec!["1.9.0".to_string(), "1.10.0".to_string(), "2.0.0-rc.1".to_string(), "2.0.0".to_string()]);
    }

    #[test]
    fn the_newest_of_a_list_is_found() {
        assert_eq!(newest(vec!["1.9.0".to_string(), "1.10.0".to_string(), "1.2.0".to_string()]).expect("all valid"), "1.10.0");
        assert!(newest(vec![]).is_err());
    }

    #[test]
    fn something_that_is_not_a_version_says_so() {
        let failure = compare("1.4".to_string(), "1.4.2".to_string()).unwrap_err();
        assert!(failure.contains("semver_compare"), "got: {}", failure);
        assert!(failure.contains("three numbers"), "got: {}", failure);

        let sort_failure = sort(vec!["1.0.0".to_string(), "nope".to_string()]).unwrap_err();
        assert!(sort_failure.contains("semver_sort"), "got: {}", sort_failure);
    }
}
