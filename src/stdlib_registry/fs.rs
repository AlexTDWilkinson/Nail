//! Filesystem module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Fs:
        "fs_read" [Tokio] => "std_lib::fs::read_file", (path: s) -> (s!e),
            "Reads an entire file into a string; errors if the file cannot be read.",
            "content:s = danger(fs_read(`notes.txt`));";
        "fs_write" [Tokio] => "std_lib::fs::write_file", (path: s, content: s) -> (v!e),
            "Writes a string to a file, creating or truncating it.",
            "danger(fs_write(`notes.txt`, content));";
        "fs_create_dir" [Tokio] => "std_lib::fs::create_dir", (path: s) -> (v!e),
            "Creates a directory and any missing parent directories.",
            "danger(fs_create_dir(`output/reports`));";
        "fs_remove_file" [Tokio] => "std_lib::fs::remove_file", (path: s) -> (v!e),
            "Deletes a file; errors if it does not exist or cannot be removed.",
            "danger(fs_remove_file(`temp.txt`));";
        "fs_copy" [Tokio] => "std_lib::fs::copy", (from: s, to: s) -> (v!e),
            "Copies a file to a new location.",
            "danger(fs_copy(`a.txt`, `b.txt`));";
        "fs_move" [Tokio] => "std_lib::fs::move_file", (from: s, to: s) -> (v!e),
            "Moves (renames) a file to a new location.",
            "danger(fs_move(`old.txt`, `new.txt`));";
    }
}
