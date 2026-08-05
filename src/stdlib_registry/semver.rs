//! Semver module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Semver:
        "semver_valid" => "std_lib::semver::valid", (version: (&s)) -> b,
            "Returns true if the text is a version number this module can read.",
            "usable:b = semver_valid(tag);";
        "semver_compare" => "std_lib::semver::compare", (first: s, second: s) -> (i!e),
            "Returns -1 if the first version is older, 0 if they are the same, and 1 if the first is newer.",
            "order:i = danger(semver_compare(`1.10.0`, `1.9.0`));";
        "semver_is_newer" => "std_lib::semver::is_newer", (first: s, second: s) -> (b!e),
            "Returns true if the first version is newer than the second.",
            "upgrade:b = danger(semver_is_newer(latest, installed));";
        "semver_is_older" => "std_lib::semver::is_older", (first: s, second: s) -> (b!e),
            "Returns true if the first version is older than the second.",
            "stale:b = danger(semver_is_older(installed, minimum));";
        "semver_major" => "std_lib::semver::major", (version: s) -> (i!e),
            "Returns the major number of a version.",
            "breaking:i = danger(semver_major(`1.4.2`));";
        "semver_minor" => "std_lib::semver::minor", (version: s) -> (i!e),
            "Returns the minor number of a version.",
            "feature:i = danger(semver_minor(`1.4.2`));";
        "semver_patch" => "std_lib::semver::patch", (version: s) -> (i!e),
            "Returns the patch number of a version.",
            "fix:i = danger(semver_patch(`1.4.2`));";
        "semver_prerelease" => "std_lib::semver::prerelease", (version: s) -> (s!e),
            "Returns the prerelease part without its hyphen, or the empty string if there is none.",
            "stage:s = danger(semver_prerelease(`1.0.0-rc.1`));";
        "semver_bump_major" => "std_lib::semver::bump_major", (version: s) -> (s!e),
            "Returns the next major version, resetting the numbers below it.",
            "next:s = danger(semver_bump_major(`1.9.3`));";
        "semver_bump_minor" => "std_lib::semver::bump_minor", (version: s) -> (s!e),
            "Returns the next minor version, resetting the patch number.",
            "next:s = danger(semver_bump_minor(`1.9.3`));";
        "semver_bump_patch" => "std_lib::semver::bump_patch", (version: s) -> (s!e),
            "Returns the next patch version.",
            "next:s = danger(semver_bump_patch(`1.9.3`));";
        "semver_satisfies" => "std_lib::semver::satisfies", (version: s, requirement: s) -> (b!e),
            "Returns true if the version meets the requirement, which may be exact, a comparison, a caret or tilde range, a star, or several of those separated by commas.",
            "allowed:b = danger(semver_satisfies(installed, `>=1.2.0, <2.0.0`));";
        "semver_sort" => "std_lib::semver::sort", (versions: [s]) -> ([s]!e),
            "Returns the versions sorted oldest first.",
            "ordered:a:s = danger(semver_sort(tags));";
        "semver_newest" => "std_lib::semver::newest", (versions: [s]) -> (s!e),
            "Returns the newest of the versions, or an error if there are none.",
            "latest:s = danger(semver_newest(tags));";
    }
}
