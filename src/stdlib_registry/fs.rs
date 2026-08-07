//! Filesystem module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Fs:
        "fs_read" [Tokio] => "std_lib::fs::read_file", (path: s) -> (s!e),
            "Reads an entire file into a string. Errors if the file cannot be read.",
            "content:s = danger(fs_read(`notes.txt`));";
        "fs_write" [Tokio] => "std_lib::fs::write_file", (path: s, content: s) -> (v!e),
            "Writes a string to a file, creating or truncating it.",
            "content:s = `remember the milk`;\ndanger(fs_write(`notes.txt`, content));";
        "fs_create_dir" [Tokio] => "std_lib::fs::create_dir", (path: s) -> (v!e),
            "Creates a directory and any missing parent directories.",
            "danger(fs_create_dir(`output/reports`));";
        "fs_remove_file" [Tokio] => "std_lib::fs::remove_file", (path: s) -> (v!e),
            "Deletes a file. Errors if it does not exist or cannot be removed.",
            "danger(fs_remove_file(`temp.txt`));";
        "fs_copy" [Tokio] => "std_lib::fs::copy", (from: s, to: s) -> (v!e),
            "Copies a file to a new location.",
            "danger(fs_copy(`a.txt`, `b.txt`));";
        "fs_move" [Tokio] => "std_lib::fs::move_file", (from: s, to: s) -> (v!e),
            "Moves (renames) a file to a new location.",
            "danger(fs_move(`old.txt`, `new.txt`));";
        "fs_append" [Tokio] => "std_lib::fs::append_file", (path: s, content: s) -> (v!e),
            "Adds to the end of a file, creating it if it is not there yet. Unlike fs_write, it keeps what the file already holds.",
            "line:s = `deploy finished`;\ndanger(fs_append(`events.log`, line));";
        "fs_read_lines" [Tokio] => "std_lib::fs::read_lines", (path: s) -> ([s]!e),
            "Reads a file and returns its lines with the line endings removed.",
            "lines:a:s = danger(fs_read_lines(`notes.txt`));";
        "fs_read_dir" [Tokio] => "std_lib::fs::read_dir", (path: s) -> ([s]!e),
            "Returns the sorted paths of everything directly inside a directory.",
            "entries:a:s = danger(fs_read_dir(`reports`));";
        "fs_walk" [Tokio] => "std_lib::fs::walk", (path: s) -> ([s]!e),
            "Returns the sorted paths of every file underneath a directory, however deep. Directories themselves are not listed and links are not followed.",
            "files:a:s = danger(fs_walk(`src`));";
        "fs_remove_dir" [Tokio] => "std_lib::fs::remove_dir", (path: s) -> (v!e),
            "Removes an empty directory. A directory with anything in it is an error.",
            "danger(fs_remove_dir(`empty_output`));";
        "fs_remove_dir_all" [Tokio] => "std_lib::fs::remove_dir_all", (path: s) -> (v!e),
            "Removes a directory and everything inside it. There is no undoing this.",
            "danger(fs_remove_dir_all(`build`));";
        "fs_size" [Tokio] => "std_lib::fs::size", (path: s) -> (i!e),
            "Returns how many bytes a file holds.",
            "bytes:i = danger(fs_size(`archive.zip`));";
        "fs_dir_size" [Tokio] => "std_lib::fs::dir_size", (path: s) -> (i!e),
            "Returns how many bytes everything under a directory adds up to, however deep - what du reports. The file sizes are added rather than the blocks they occupy, and links are not followed.",
            "used:i = danger(fs_dir_size(`uploads`));";
        "fs_files_equal" [Tokio] => "std_lib::fs::files_equal", (first_path: s, second_path: s) -> (b!e),
            "Returns whether two files hold exactly the same bytes. Different lengths answer without reading either file, and files that differ early stop there rather than reading to the end.",
            "same:b = danger(fs_files_equal(`build/out.js`, `dist/out.js`));";
        "fs_modified" [Tokio] => "std_lib::fs::modified", (path: s) -> (i!e),
            "Returns when a file was last changed, as a Unix timestamp in seconds to compare with time_now.",
            "changed:i = danger(fs_modified(`notes.txt`));";
        "fs_is_dir" [Tokio] => "std_lib::fs::is_dir", (path: s) -> (b!e),
            "Returns whether the path names a directory. False for a file and false for a path that is not there. A path that cannot be looked at, such as one inside a directory you may not read, is an error rather than false.",
            "folder:b = danger(fs_is_dir(`reports`));";
        "fs_is_file" [Tokio] => "std_lib::fs::is_file", (path: s) -> (b!e),
            "Returns whether the path names a file. False for a directory and false for a path that is not there. A path that cannot be looked at is an error rather than false.",
            "regular:b = danger(fs_is_file(`notes.txt`));";
        "fs_write_atomic" [Tokio] => "std_lib::fs::write_atomic", (path: s, content: s) -> (v!e),
            "Writes a file by writing beside it and renaming into place, so a reader never sees a half-written file and a crash leaves the old one intact. The way to write a config, cache or state file.",
            "encoded:s = `{\"page\":2}`;\ndanger(fs_write_atomic(`state.json`, encoded));";
        "fs_temp_file" [Tokio] => "std_lib::fs::temp_file", (prefix: s, extension: s) -> (s!e),
            "Creates a new empty file nobody else has in the temporary directory and returns its path, carrying the prefix and extension given.",
            "scratch:s = danger(fs_temp_file(`export_`, `csv`));";
        "fs_set_executable" [Tokio] => "std_lib::fs::set_executable", (path: s, executable: b) -> (v!e),
            "Turns the executable bit on or off for a file - the step a program that writes a script has to take before it can run it.",
            "danger(fs_set_executable(`build.sh`, true));";
        "fs_is_executable" [Tokio] => "std_lib::fs::is_executable", (path: s) -> (b!e),
            "Whether a file can be run as a program. False for a directory or a missing file. A path that cannot be looked at is an error rather than false.",
            "runnable:b = danger(fs_is_executable(`build.sh`));";
        "fs_temp_dir" [Tokio] => "std_lib::fs::temp_dir", () -> s,
            "Returns the directory this machine keeps temporary files in. Nothing is created - join a name onto it with path_join.",
            "scratch:s = fs_temp_dir();";
        "fs_glob" [Tokio] => "std_lib::fs::glob", (directory: s, pattern: s) -> ([s]!e),
            "Returns every file at or below the directory whose path matches the glob pattern, sorted. The pattern is matched against the path below the directory.",
            "sources:a:s = danger(fs_glob(`src`, `**/*.rs`));";
        "fs_read_base64" [Tokio, Base64] => "std_lib::fs::read_base64", (path: s) -> (s!e),
            "A file's contents as base64 text - the way to get a file that is not text into a program, for a data: URI or a JSON field. A third larger than the bytes, so for small files.",
            "encoded:s = danger(fs_read_base64(`logo.png`));";
        "fs_write_base64" [Tokio, Base64] => "std_lib::fs::write_base64", (path: s, data: s) -> (v!e),
            "Writes base64 text back out as the bytes it stands for. Text that is not base64 is an error rather than a file full of nonsense.",
            "encoded:s = base64_encode(`a tiny file`);\ndanger(fs_write_base64(`logo.png`, encoded));";
        "fs_append_file" [Tokio] => "std_lib::fs::append_from_file", (from_path: s, to_path: s) -> (v!e),
            "Adds one file to the end of another, copying in blocks so neither has to fit in memory. How the pieces of a resumable upload are put back together.",
            "danger(fs_append_file(`part_2.bin`, `whole.bin`));";
        "fs_read_range_base64" [Tokio, Base64] => "std_lib::fs::read_range_base64", (path: s, offset: i, length: i) -> (s!e),
            "A slice of a file as base64, for looking inside one without loading it. Fewer bytes come back if the file ends first.",
            "header:s = danger(fs_read_range_base64(`upload.bin`, 0, 64));";
        "fs_read_range_hex" [Tokio] => "std_lib::fs::read_range_hex", (path: s, offset: i, length: i) -> (s!e),
            "A slice of a file as hex, which is how a program works out what a file is: a PNG starts 89504e47, a zip 504b0304.",
            "magic:s = danger(fs_read_range_hex(`upload.bin`, 0, 4));";
    }

    let reader_parameter = || StdlibParameter { name: "reader".to_string(), param_type: NailDataTypeDescriptor::Struct("FS_Reader".to_string()), pass_by_reference: true };
    let reader_import = || vec![("FS_Reader", "nail::std_lib::fs")];

    m.insert("fs_open", StdlibFunction {
        rust_path: "std_lib::fs::open_reader".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::Uuid, CrateDependency::DashMap, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: reader_import(),
        module: StdlibModule::Fs,
        parameters: vec![nail_param!(path: s)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("FS_Reader".to_string()))),
        diverging: false,
        description: "Opens a file for reading a piece at a time, for a file too large to hold in memory. Closed by fs_close, and closes itself once it reaches the end.",
        example: "reader:FS_Reader = danger(fs_open(`app.log`));",
    });

    m.insert("fs_next_lines", StdlibFunction {
        rust_path: "std_lib::fs::next_lines".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::DashMap, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: reader_import(),
        module: StdlibModule::Fs,
        parameters: vec![reader_parameter(), nail_param!(count: i)],
        return_type: nail_type!(([s]!e)),
        diverging: false,
        description: "The next lines from an open reader, without their line endings - at most count of them, and fewer at the end. An empty array means the file is finished and the reader has closed itself.",
        example: "reader:FS_Reader = danger(fs_open(`app.log`));\nlines:a:s = danger(fs_next_lines(reader, 1000));",
    });

    m.insert("fs_close", StdlibFunction {
        rust_path: "std_lib::fs::close_reader".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::DashMap, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: reader_import(),
        module: StdlibModule::Fs,
        parameters: vec![reader_parameter()],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Closes a reader. Closing one that already reached the end is not an error.",
        example: "reader:FS_Reader = danger(fs_open(`app.log`));\ndanger(fs_close(reader));",
    });

    m.insert("fs_reduce_lines", StdlibFunction {
        rust_path: "std_lib::fs::open_reader".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::Uuid, CrateDependency::DashMap, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: reader_import(),
        module: StdlibModule::Fs,
        parameters: vec![
            nail_param!(path: s),
            StdlibParameter { name: "initial".to_string(), param_type: NailDataTypeDescriptor::TypeVar("A".to_string(), vec![]), pass_by_reference: false },
            StdlibParameter {
                name: "step".to_string(),
                param_type: NailDataTypeDescriptor::Fn(
                    vec![NailDataTypeDescriptor::TypeVar("A".to_string(), vec![]), NailDataTypeDescriptor::String],
                    Box::new(NailDataTypeDescriptor::TypeVar("A".to_string(), vec![])),
                ),
                pass_by_reference: false,
            },
        ],
        return_type: nail_type!((A!e)),
        diverging: false,
        description: "Reads a file a line at a time and folds it into one value, the way reduce folds an array - so a file larger than memory can be counted, summed or searched. The step function takes what has been accumulated so far and the next line, and may read files or make requests itself.",
        example: "f count_errors(seen:i, line:s):i {\n    r if {\n        string_contains(line, `ERROR`) -> { r seen + 1; },\n        else -> { r seen; }\n    };\n}\n\nerrors:i = danger(fs_reduce_lines(`app.log`, 0, count_errors));",
    });

    let watcher_parameter = || StdlibParameter { name: "watcher".to_string(), param_type: NailDataTypeDescriptor::Struct("FS_Watcher".to_string()), pass_by_reference: true };
    let watcher_import = || vec![("FS_Watcher", "nail::std_lib::fs")];

    m.insert("fs_watch_start", StdlibFunction {
        rust_path: "std_lib::fs::watch_start".to_string(),
        crate_deps: vec![CrateDependency::Notify, CrateDependency::Tokio, CrateDependency::Uuid, CrateDependency::DashMap, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: watcher_import(),
        module: StdlibModule::Fs,
        parameters: vec![nail_param!(path: s)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("FS_Watcher".to_string()))),
        diverging: false,
        description: "Starts watching a file or directory for changes, directories all the way down. Changes pile up until fs_watch_next collects them, so nothing is missed between calls.",
        example: "watcher:FS_Watcher = danger(fs_watch_start(`src`));",
    });

    m.insert("fs_watch_next", StdlibFunction {
        rust_path: "std_lib::fs::watch_next".to_string(),
        crate_deps: vec![CrateDependency::Notify, CrateDependency::Tokio, CrateDependency::DashMap, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: watcher_import(),
        module: StdlibModule::Fs,
        parameters: vec![watcher_parameter(), nail_param!(timeout_milliseconds: i)],
        return_type: nail_type!(([s]!e)),
        diverging: false,
        description: "The paths that changed since the last call, waiting up to the timeout for the first change. An empty array means the time passed quietly.",
        example: "watcher:FS_Watcher = danger(fs_watch_start(`src`));\nchanged:a:s = danger(fs_watch_next(watcher, 5000));",
    });

    m.insert("fs_watch_stop", StdlibFunction {
        rust_path: "std_lib::fs::watch_stop".to_string(),
        crate_deps: vec![CrateDependency::Notify, CrateDependency::Tokio, CrateDependency::DashMap, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: watcher_import(),
        module: StdlibModule::Fs,
        parameters: vec![watcher_parameter()],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Ends a watch and forgets its handle. Stopping one twice is not an error.",
        example: "watcher:FS_Watcher = danger(fs_watch_start(`src`));\ndanger(fs_watch_stop(watcher));",
    });

    simple_fns! { m, Fs:
        "fs_read_with_encoding" [EncodingRs, Tokio] => "std_lib::fs::read_with_encoding", (path: s, encoding_label: s) -> (s!e),
            "Reads a file that is not UTF-8 - the windows-1252 CSV a bank exports, the shift_jis page an old site serves. Labels are WHATWG style: `windows-1252`, `shift_jis`, `utf-16le`, `euc-kr`.",
            "text:s = danger(fs_read_with_encoding(`export.csv`, `windows-1252`));";
        "fs_tail_lines" [Tokio] => "std_lib::fs::tail_lines", (path: s, count: i) -> ([s]!e),
            "The last lines of a file, read from the end - how a person looks at a log. Walked backwards in blocks, so a huge file costs only as much as the lines asked for.",
            "recent:a:s = danger(fs_tail_lines(`app.log`, 50));";
    }
}
