//! Path module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Path:
        "path_join" => "std_lib::path::join", (base: s, path: s) -> s,
            "Joins two path segments with the platform separator.",
            "full:s = path_join(`/home/user`, `notes.txt`);";
        "path_exists" => "std_lib::path::exists", (path: s) -> b,
            "Returns true if a file or directory exists at the path.",
            "present:b = path_exists(`config.toml`);";
        "path_basename" => "std_lib::path::basename", (path: s) -> s,
            "Returns the final component of the path (the file or directory name).",
            "name:s = path_basename(`/tmp/report.pdf`);";
        "path_dirname" => "std_lib::path::dirname", (path: s) -> s,
            "Returns the path without its final component.",
            "dir:s = path_dirname(`/tmp/report.pdf`);";
        "path_extension" => "std_lib::path::extension", (path: s) -> (s!e),
            "Returns the file extension without the dot. Errors if there is none.",
            "ext:s = danger(path_extension(`report.pdf`));";
        "path_is_absolute" => "std_lib::path::is_absolute", (path: s) -> b,
            "Returns true if the path is absolute.",
            "absolute:b = path_is_absolute(`/etc/hosts`);";
        "path_normalize" => "std_lib::path::normalize", (path: s) -> s,
            "Normalizes a path by resolving . and .. components.",
            "clean:s = path_normalize(`a/./b/../c`);";
        "path_stem" => "std_lib::path::stem", (path: s) -> s,
            "Returns the file name with its last extension removed, so report.tar.gz gives report.tar.",
            "name:s = path_stem(`/tmp/report.pdf`);";
        "path_relative_to" => "std_lib::path::relative_to", (path: s, base: s) -> (s!e),
            "Returns the path written from the base directory instead. Errors if the path is not inside the base.",
            "inside:s = danger(path_relative_to(`/srv/app/logs/today.log`, `/srv/app`));";
        "path_absolute" => "std_lib::path::absolute", (path: s) -> (s!e),
            "Returns the path resolved against the directory the program is running in. Works for a file that does not exist yet.",
            "full:s = danger(path_absolute(`config.toml`));";
        "path_with_extension" => "std_lib::path::with_extension", (path: s, extension: s) -> s,
            "Returns the same path carrying a different extension. An empty extension removes it.",
            "output:s = path_with_extension(`report.md`, `html`);";
        "path_matches_glob" => "std_lib::path::matches_glob", (pattern: (&s), path: (&s)) -> b,
            "Whether a path matches a shell glob pattern, where * stays inside one segment, ** crosses segments, ? is one character and [abc] is one of those listed.",
            "path:s = `tests/parser/test_arrays.nail`;\nis_test:b = path_matches_glob(`tests/**/*.nail`, path);";
        "path_depth" => "std_lib::path::depth", (path: s) -> i,
            "Returns how many segments the path has below its root or its start, counted after normalizing, so a/b/c.txt is 3 and / is 0.",
            "levels:i = path_depth(`a/b/c.txt`);";
        "path_segments" => "std_lib::path::segments", (path: s) -> [s],
            "Returns the pieces of the path between separators, with the empties a doubled or trailing separator would make dropped.",
            "parts:a:s = path_segments(`/srv/app/logs`);";
        "path_is_hidden" => "std_lib::path::is_hidden", (path: s) -> b,
            "Returns true when the file name starts with a dot, judged on the final component only.",
            "hidden:b = path_is_hidden(`.env`);";
        "path_within" => "std_lib::path::within", (base: s, candidate: s) -> b,
            "Returns true when the candidate stays inside the base once both are normalized, so a .. that climbs out is caught. Pure string work, no filesystem access.",
            "requested:s = `/srv/files/report.pdf`;\ninside:b = path_within(`/srv/files`, requested);";
        "path_with_stem" => "std_lib::path::with_stem", (path: s, stem: s) -> (s!e),
            "Returns the path with a different file name but the same directory and extension. Errors on an empty stem or a path with no file name.",
            "renamed:s = danger(path_with_stem(`logs/app.log`, `backup`));";
        "path_sanitize_filename" => "std_lib::path::sanitize_filename", (name: s) -> s,
            "Makes untrusted text safe as a single file name: separators, dot-dot, control characters, characters Windows refuses and leading dots become underscores, and empty input gives file.",
            "upload_name:s = `../../etc/passwd`;\nstored:s = path_sanitize_filename(upload_name);";
        "path_common_prefix" => "std_lib::path::common_prefix", (paths: [s]) -> s,
            "Returns the longest directory prefix the paths share, whole segments only, and an empty string when nothing is shared.",
            "files:a:s = [`/srv/app/one.nail`, `/srv/app/two.nail`];\nshared:s = path_common_prefix(files);";
    }
}
