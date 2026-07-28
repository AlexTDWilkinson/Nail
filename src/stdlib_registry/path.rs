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
            "Returns the file extension without the dot; errors if there is none.",
            "ext:s = danger(path_extension(`report.pdf`));";
        "path_is_absolute" => "std_lib::path::is_absolute", (path: s) -> b,
            "Returns true if the path is absolute.",
            "absolute:b = path_is_absolute(`/etc/hosts`);";
        "path_normalize" => "std_lib::path::normalize", (path: s) -> s,
            "Normalizes a path by resolving . and .. components.",
            "clean:s = path_normalize(`a/./b/../c`);";
    }
}
