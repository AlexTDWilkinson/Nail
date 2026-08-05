use std::path::Path;

pub async fn read_file(path: String) -> Result<String, String> {
    tokio::fs::read_to_string(Path::new(&path))
        .await
        .map_err(|e| format!("fs_read: could not read file '{}': {}", path, e))
}

pub async fn write_file(path: String, content: String) -> Result<(), String> {
    tokio::fs::write(Path::new(&path), content)
        .await
        .map_err(|e| format!("fs_write: could not write file '{}': {}", path, e))
}

pub async fn create_dir(path: String) -> Result<(), String> {
    tokio::fs::create_dir_all(Path::new(&path))
        .await
        .map_err(|e| format!("fs_create_dir: could not create directory '{}': {}", path, e))
}

pub async fn remove_file(path: String) -> Result<(), String> {
    tokio::fs::remove_file(Path::new(&path))
        .await
        .map_err(|e| format!("fs_remove_file: could not remove file '{}': {}", path, e))
}

pub async fn copy(from: String, to: String) -> Result<(), String> {
    tokio::fs::copy(Path::new(&from), Path::new(&to))
        .await
        .map(|_| ())
        .map_err(|e| format!("fs_copy: could not copy '{}' to '{}': {}", from, to, e))
}

pub async fn move_file(from: String, to: String) -> Result<(), String> {
    tokio::fs::rename(Path::new(&from), Path::new(&to))
        .await
        .map_err(|e| format!("fs_move: could not move '{}' to '{}': {}", from, to, e))
}

/// Adds to the end of a file, creating it if it is not there yet. What a log
/// file or an append-only record wants; `fs_write` would throw away what is
/// already in the file.
pub async fn append_file(path: String, content: String) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(Path::new(&path))
        .await
        .map_err(|e| format!("fs_append: could not open file '{}': {}", path, e))?;
    return file.write_all(content.as_bytes()).await.map_err(|e| format!("fs_append: could not write to file '{}': {}", path, e));
}

/// The file split into lines, with the line endings removed. A trailing
/// newline does not produce an empty last line, because a file ending in a
/// newline has no line after it.
pub async fn read_lines(path: String) -> Result<Vec<String>, String> {
    let content = tokio::fs::read_to_string(Path::new(&path))
        .await
        .map_err(|e| format!("fs_read_lines: could not read file '{}': {}", path, e))?;
    return Ok(content.lines().map(|line| line.to_string()).collect());
}

/// Everything directly inside a directory, as paths rather than bare names, so
/// the answers can be passed straight back to another filesystem function.
/// Sorted, because the order the operating system hands them back in is
/// arbitrary and a program that depends on it is a program that breaks on
/// another machine.
pub async fn read_dir(path: String) -> Result<Vec<String>, String> {
    let mut entries = tokio::fs::read_dir(Path::new(&path))
        .await
        .map_err(|e| format!("fs_read_dir: could not read directory '{}': {}", path, e))?;

    let mut found = Vec::new();
    loop {
        let entry = entries.next_entry().await.map_err(|e| format!("fs_read_dir: could not read directory '{}': {}", path, e))?;
        match entry {
            Some(entry) => found.push(entry.path().to_string_lossy().to_string()),
            None => break,
        }
    }
    found.sort();
    return Ok(found);
}

/// Every file underneath a directory, however deep, as sorted paths.
/// Directories themselves are not in the list - they are the road, not the
/// destination. Symbolic links are not followed, so a link pointing at its own
/// parent cannot send this round forever.
pub async fn walk(path: String) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    let mut pending = vec![path.clone()];

    while let Some(directory) = pending.pop() {
        let mut entries = tokio::fs::read_dir(Path::new(&directory))
            .await
            .map_err(|e| format!("fs_walk: could not read directory '{}': {}", directory, e))?;

        loop {
            let entry = entries.next_entry().await.map_err(|e| format!("fs_walk: could not read directory '{}': {}", directory, e))?;
            let entry = match entry {
                Some(entry) => entry,
                None => break,
            };

            let entry_path = entry.path().to_string_lossy().to_string();
            let file_type = entry.file_type().await.map_err(|e| format!("fs_walk: could not inspect '{}': {}", entry_path, e))?;
            if file_type.is_dir() {
                pending.push(entry_path);
            } else if file_type.is_file() {
                found.push(entry_path);
            }
        }
    }

    found.sort();
    return Ok(found);
}

/// Removes an empty directory. A directory with anything in it is an error,
/// so a mistyped path cannot quietly delete a tree; `fs_remove_dir_all` is the
/// one that means it.
pub async fn remove_dir(path: String) -> Result<(), String> {
    tokio::fs::remove_dir(Path::new(&path))
        .await
        .map_err(|e| format!("fs_remove_dir: could not remove directory '{}': {}", path, e))
}

/// Removes a directory and everything inside it. There is no undoing this.
pub async fn remove_dir_all(path: String) -> Result<(), String> {
    tokio::fs::remove_dir_all(Path::new(&path))
        .await
        .map_err(|e| format!("fs_remove_dir_all: could not remove directory '{}': {}", path, e))
}

/// How many bytes a file holds.
pub async fn size(path: String) -> Result<i64, String> {
    let metadata = tokio::fs::metadata(Path::new(&path)).await.map_err(|e| format!("fs_size: could not inspect '{}': {}", path, e))?;
    return Ok(metadata.len() as i64);
}

/// When a file was last changed, as a Unix timestamp in seconds, to compare
/// with `time_now`. Some filesystems do not record this, and that is an error
/// rather than a made-up date.
pub async fn modified(path: String) -> Result<i64, String> {
    let metadata = tokio::fs::metadata(Path::new(&path)).await.map_err(|e| format!("fs_modified: could not inspect '{}': {}", path, e))?;
    let changed = metadata.modified().map_err(|e| format!("fs_modified: this filesystem does not record when '{}' changed: {}", path, e))?;
    return match changed.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => Ok(duration.as_secs() as i64),
        Err(e) => Ok(-(e.duration().as_secs() as i64)),
    };
}

/// Whether the path names a directory. False for a file, and false for a path
/// that is not there at all.
pub async fn is_dir(path: String) -> bool {
    return match tokio::fs::metadata(Path::new(&path)).await {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    };
}

/// Whether the path names a file. False for a directory, and false for a path
/// that is not there at all.
pub async fn is_file(path: String) -> bool {
    return match tokio::fs::metadata(Path::new(&path)).await {
        Ok(metadata) => metadata.is_file(),
        Err(_) => false,
    };
}

/// The directory this machine keeps temporary files in. Nothing is created
/// here - join a name onto it with `path_join` first.
pub async fn temp_dir() -> String {
    return std::env::temp_dir().to_string_lossy().to_string();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory of this test's own, named after the check running in it so
    /// two tests never share one.
    async fn scratch(name: &str) -> String {
        let directory = format!("{}/nail_fs_tests/{}", std::env::temp_dir().to_string_lossy(), name);
        let _ = tokio::fs::remove_dir_all(&directory).await;
        create_dir(directory.clone()).await.expect("a fresh directory");
        return directory;
    }

    #[tokio::test]
    async fn appending_adds_to_the_end_and_creates_what_is_missing() {
        let directory = scratch("append").await;
        let path = format!("{}/log.txt", directory);

        append_file(path.clone(), "one\n".to_string()).await.expect("a writable path");
        append_file(path.clone(), "two\n".to_string()).await.expect("a writable path");

        assert_eq!(read_file(path).await.expect("a written file"), "one\ntwo\n");
        remove_dir_all(directory).await.expect("a removable directory");
    }

    #[tokio::test]
    async fn lines_come_back_without_their_endings() {
        let directory = scratch("lines").await;
        let path = format!("{}/notes.txt", directory);
        write_file(path.clone(), "first\nsecond\nthird\n".to_string()).await.expect("a writable path");

        let lines = read_lines(path).await.expect("a written file");
        assert_eq!(lines, vec!["first".to_string(), "second".to_string(), "third".to_string()]);
        remove_dir_all(directory).await.expect("a removable directory");
    }

    #[tokio::test]
    async fn reading_a_directory_lists_what_is_directly_inside_it() {
        let directory = scratch("read_dir").await;
        write_file(format!("{}/b.txt", directory), "b".to_string()).await.expect("a writable path");
        write_file(format!("{}/a.txt", directory), "a".to_string()).await.expect("a writable path");
        create_dir(format!("{}/nested", directory)).await.expect("a writable path");
        write_file(format!("{}/nested/deep.txt", directory), "deep".to_string()).await.expect("a writable path");

        let entries = read_dir(directory.clone()).await.expect("a real directory");
        assert_eq!(entries.len(), 3, "the nested file must not be listed: {:?}", entries);
        assert!(entries[0].ends_with("a.txt"), "entries come back sorted: {:?}", entries);

        remove_dir_all(directory).await.expect("a removable directory");
    }

    #[tokio::test]
    async fn walking_finds_every_file_however_deep_and_no_directories() {
        let directory = scratch("walk").await;
        create_dir(format!("{}/one/two", directory)).await.expect("a writable path");
        write_file(format!("{}/top.txt", directory), "top".to_string()).await.expect("a writable path");
        write_file(format!("{}/one/middle.txt", directory), "middle".to_string()).await.expect("a writable path");
        write_file(format!("{}/one/two/bottom.txt", directory), "bottom".to_string()).await.expect("a writable path");

        let found = walk(directory.clone()).await.expect("a real directory");
        assert_eq!(found.len(), 3, "expected three files, got {:?}", found);
        assert!(found[0].ends_with("one/middle.txt"), "results come back sorted: {:?}", found);

        remove_dir_all(directory).await.expect("a removable directory");
    }

    #[tokio::test]
    async fn a_directory_with_something_in_it_will_not_be_removed_by_accident() {
        let directory = scratch("remove").await;
        write_file(format!("{}/keep.txt", directory), "keep".to_string()).await.expect("a writable path");

        assert!(remove_dir(directory.clone()).await.unwrap_err().contains("could not remove directory"));
        remove_dir_all(directory.clone()).await.expect("a removable directory");
        assert!(!is_dir(directory).await);
    }

    #[tokio::test]
    async fn size_and_kind_describe_what_is_there() {
        let directory = scratch("metadata").await;
        let path = format!("{}/five.txt", directory);
        write_file(path.clone(), "12345".to_string()).await.expect("a writable path");

        assert_eq!(size(path.clone()).await.expect("a written file"), 5);
        assert!(is_file(path.clone()).await);
        assert!(!is_dir(path.clone()).await);
        assert!(is_dir(directory.clone()).await);
        assert!(modified(path).await.expect("a written file") > 0);

        remove_dir_all(directory).await.expect("a removable directory");
    }

    #[tokio::test]
    async fn a_path_that_is_not_there_is_neither_file_nor_directory() {
        let missing = format!("{}/nail_fs_tests/nothing_here_at_all", std::env::temp_dir().to_string_lossy());
        assert!(!is_file(missing.clone()).await);
        assert!(!is_dir(missing.clone()).await);
        assert!(size(missing).await.unwrap_err().contains("could not inspect"));
    }
}
/// Every file at or below a directory whose path matches the glob pattern,
/// sorted. This is `fs_walk` with the pattern applied, which is the loop every
/// program that works over a set of files was writing by hand.
///
/// The pattern is matched against the whole path as walked, so a pattern that
/// starts at the search directory - `tests/**/*.nail` under `tests` - matches
/// nothing; write the part below the directory instead.
pub async fn glob(directory: String, pattern: String) -> Result<Vec<String>, String> {
    let root = directory.trim_end_matches('/').to_string();
    let found = walk(root.clone()).await.map_err(|detail| detail.replace("fs_walk", "fs_glob"))?;
    let prefix = format!("{}/", root);
    let mut matching: Vec<String> = found
        .into_iter()
        .filter(|path| {
            let relative = path.strip_prefix(&prefix).unwrap_or(path).to_string();
            return crate::parser::std_lib::path::matches_glob(&pattern, &relative);
        })
        .collect();
    matching.sort();
    return Ok(matching);
}

/// A file's contents as base64 text. The way to get a file that is not text into
/// a Nail program at all: an image to embed in a page as a `data:` URI, a small
/// binary to send in a JSON field.
///
/// Base64 is a third larger than the bytes it stands for, so this is for the
/// small things it is worth doing to. Copying a file is `fs_copy`, hashing one is
/// `crypto_hash_file_sha256`, and neither reads it into the program.
pub async fn read_base64(path: String) -> Result<String, String> {
    use base64::Engine;
    let bytes = tokio::fs::read(&path).await.map_err(|failure| format!("fs_read_base64: could not read '{}': {}", path, failure))?;
    return Ok(base64::engine::general_purpose::STANDARD.encode(bytes));
}

/// Writes base64 text back out as the bytes it stands for, creating or replacing
/// the file. Text that is not base64 is an error rather than a file full of
/// nonsense.
pub async fn write_base64(path: String, data: String) -> Result<(), String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data.trim())
        .map_err(|failure| format!("fs_write_base64: the text given for '{}' is not base64: {}", path, failure))?;
    tokio::fs::write(&path, bytes).await.map_err(|failure| format!("fs_write_base64: could not write '{}': {}", path, failure))?;
    return Ok(());
}

#[cfg(test)]
mod base64_file_tests {
    use super::*;

    #[tokio::test]
    async fn bytes_survive_a_round_trip_through_base64() {
        let original = std::env::temp_dir().join("nail_base64_round_trip.bin");
        let copy = std::env::temp_dir().join("nail_base64_round_trip_copy.bin");
        // Bytes that are not valid UTF-8, which is the case this exists for.
        let contents: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0xff, 0xd8, 0x00, 0x01];
        std::fs::write(&original, &contents).expect("a writable file");

        let encoded = read_base64(original.to_string_lossy().to_string()).await.expect("a readable file");
        assert!(!encoded.is_empty());
        write_base64(copy.to_string_lossy().to_string(), encoded).await.expect("a writable file");
        assert_eq!(std::fs::read(&copy).expect("the copy"), contents);

        let _ = std::fs::remove_file(&original);
        let _ = std::fs::remove_file(&copy);
    }

    #[tokio::test]
    async fn text_that_is_not_base64_is_refused_rather_than_written() {
        let path = std::env::temp_dir().join("nail_base64_never_written.bin");
        let _ = std::fs::remove_file(&path);
        let failure = write_base64(path.to_string_lossy().to_string(), "not base64!!".to_string()).await.unwrap_err();
        assert!(failure.contains("is not base64"), "got: {}", failure);
        assert!(!path.exists(), "nothing should have been written");
    }

    #[tokio::test]
    async fn a_file_that_is_not_there_says_so() {
        let failure = read_base64("/tmp/nail_no_such_file_base64".to_string()).await.unwrap_err();
        assert!(failure.contains("fs_read_base64"), "got: {}", failure);
    }
}

/// Writes a file the safe way: the content goes to a temporary file beside the
/// destination first, and only a rename puts it in place. A rename within one
/// directory either happens or does not, so a reader of that path never sees a
/// half-written file, and a crash mid-write leaves the old one intact. This is
/// how a config file, a cache or a lock file should be written every time.
pub async fn write_atomic(path: String, content: String) -> Result<(), String> {
    let destination = Path::new(&path);
    let directory = destination.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let name = destination.file_name().and_then(|name| name.to_str()).ok_or_else(|| format!("fs_write_atomic: '{}' does not name a file", path))?;

    let temporary = directory.join(format!(".{}.{}.partial", name, std::process::id()));
    tokio::fs::write(&temporary, content).await.map_err(|e| format!("fs_write_atomic: could not write next to '{}': {}", path, e))?;

    if let Err(error) = tokio::fs::rename(&temporary, destination).await {
        // Leaving the partial file behind would be worse than the failure.
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(format!("fs_write_atomic: could not put '{}' in place: {}", path, error));
    }
    return Ok(());
}

/// Creates a new empty file nobody else has, in the system temporary
/// directory, and returns its path. The name carries the prefix and extension
/// you ask for so it is recognisable in a directory listing, and the process id
/// and a counter so two runs - or two calls - never collide.
pub async fn temp_file(prefix: String, extension: String) -> Result<String, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    let suffix = match extension.trim_start_matches('.') {
        "" => String::new(),
        wanted => format!(".{}", wanted),
    };
    let directory = std::env::temp_dir();

    // A file that already exists is somebody else's; take the next number.
    for _ in 0..1000 {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let candidate = directory.join(format!("{}{}_{}{}", prefix, std::process::id(), sequence, suffix));
        match tokio::fs::OpenOptions::new().write(true).create_new(true).open(&candidate).await {
            Ok(_) => return Ok(candidate.to_string_lossy().to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("fs_temp_file: could not create a temporary file in '{}': {}", directory.to_string_lossy(), error)),
        }
    }
    return Err(format!("fs_temp_file: could not find an unused name in '{}'", directory.to_string_lossy()));
}

/// Turns the executable bit on or off for a file. The step every program that
/// writes a script or ships a binary forgets, and then cannot run what it just
/// wrote. Does nothing on systems without file modes.
pub async fn set_executable(path: String, executable: bool) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = tokio::fs::metadata(Path::new(&path)).await.map_err(|e| format!("fs_set_executable: could not read '{}': {}", path, e))?;
        let mut permissions = metadata.permissions();
        // Follows the read bits: what can be read can be run.
        let readable = permissions.mode() & 0o444;
        let mode = if executable { permissions.mode() | (readable >> 2) } else { permissions.mode() & !0o111 };
        permissions.set_mode(mode);
        tokio::fs::set_permissions(Path::new(&path), permissions).await.map_err(|e| format!("fs_set_executable: could not change the permissions of '{}': {}", path, e))?;
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        let _ = (path, executable);
        return Ok(());
    }
}

/// Whether a file can be run as a program. False for a directory, and false on
/// systems without file modes.
pub async fn is_executable(path: String) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return match tokio::fs::metadata(Path::new(&path)).await {
            Ok(metadata) => metadata.is_file() && metadata.permissions().mode() & 0o111 != 0,
            Err(_) => false,
        };
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        return false;
    }
}

#[cfg(test)]
mod writing_tests {
    use super::*;

    #[tokio::test]
    async fn an_atomic_write_leaves_no_partial_file_behind() {
        let path = temp_file("nail_atomic_".to_string(), "txt".to_string()).await.expect("a writable temporary directory");
        write_atomic(path.clone(), "final".to_string()).await.expect("a writable path");
        assert_eq!(read_file(path.clone()).await.expect("the file we wrote"), "final");

        let directory = Path::new(&path).parent().expect("a parent").to_path_buf();
        let name = Path::new(&path).file_name().expect("a name").to_string_lossy().to_string();
        let partial = directory.join(format!(".{}.{}.partial", name, std::process::id()));
        assert!(!partial.exists());

        tokio::fs::remove_file(&path).await.expect("a removable file");
    }

    #[tokio::test]
    async fn an_atomic_write_to_a_directory_that_is_not_there_is_an_error() {
        let error = write_atomic("/nowhere/at/all/config.toml".to_string(), "x".to_string()).await.unwrap_err();
        assert!(error.contains("could not write next to"));
    }

    #[tokio::test]
    async fn every_temporary_file_is_new_and_carries_the_name_it_was_given() {
        let first = temp_file("nail_unique_".to_string(), ".log".to_string()).await.expect("a writable temporary directory");
        let second = temp_file("nail_unique_".to_string(), "log".to_string()).await.expect("a writable temporary directory");
        assert_ne!(first, second);
        assert!(first.contains("nail_unique_"));
        assert!(first.ends_with(".log"));
        assert!(second.ends_with(".log"));
        assert_eq!(read_file(first.clone()).await.expect("an empty file"), "");

        tokio::fs::remove_file(&first).await.expect("a removable file");
        tokio::fs::remove_file(&second).await.expect("a removable file");
    }

    #[tokio::test]
    async fn the_executable_bit_goes_on_and_off() {
        let path = temp_file("nail_script_".to_string(), "sh".to_string()).await.expect("a writable temporary directory");
        write_file(path.clone(), "#!/bin/sh\necho hi\n".to_string()).await.expect("a writable path");

        assert!(!is_executable(path.clone()).await);
        set_executable(path.clone(), true).await.expect("a file we own");
        assert!(is_executable(path.clone()).await);
        set_executable(path.clone(), false).await.expect("a file we own");
        assert!(!is_executable(path.clone()).await);

        tokio::fs::remove_file(&path).await.expect("a removable file");
    }

    #[tokio::test]
    async fn a_file_that_is_not_there_is_not_executable() {
        assert!(!is_executable("/nowhere/at/all".to_string()).await);
        assert!(set_executable("/nowhere/at/all".to_string(), true).await.unwrap_err().contains("could not read"));
    }
}

/// An open reader, kept by handle the way a database connection is: the thing
/// itself has a file descriptor and a buffer in it, which is not something a Nail
/// value can hold, so the program holds a name for it instead.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FS_Reader {
    pub handle: String,
    pub path: String,
}

lazy_static::lazy_static! {
    static ref OPEN_READERS: dashmap::DashMap<String, tokio::io::BufReader<tokio::fs::File>> = dashmap::DashMap::new();
}

/// Opens a file for reading a piece at a time, for a file too large to hold. The
/// reader is closed by `fs_close`, and closes itself once it reaches the end, so
/// the only way to leave one open is to abandon it half-read.
pub async fn open_reader(path: String) -> Result<FS_Reader, String> {
    let file = tokio::fs::File::open(&path).await.map_err(|failure| format!("fs_open: could not read '{}': {}", path, failure))?;
    let handle = format!("fs_reader_{}", uuid::Uuid::new_v4());
    OPEN_READERS.insert(handle.clone(), tokio::io::BufReader::new(file));
    return Ok(FS_Reader { handle, path });
}

/// The next lines from an open reader, without their line endings - at most
/// `count` of them, and fewer at the end of the file.
///
/// An empty array means the file is finished, and the reader has closed itself by
/// the time it is returned. So the shape of a read loop is: ask for lines, stop
/// when there are none, and nothing needs closing on that path.
pub async fn next_lines(reader: &FS_Reader, count: i64) -> Result<Vec<String>, String> {
    use tokio::io::AsyncBufReadExt;

    if count < 1 {
        return Err(format!("fs_next_lines: asked for {} lines, which is not a number of lines to read", count));
    }
    let mut open = match OPEN_READERS.get_mut(&reader.handle) {
        Some(open) => open,
        // Reading past the end is not an error: the reader closed itself, and a
        // loop that asks once more gets the same answer it got last time.
        None => return Ok(Vec::new()),
    };

    let mut lines = Vec::with_capacity(count as usize);
    let mut at_end = false;
    for _ in 0..count {
        let mut line = String::new();
        let read = open.read_line(&mut line).await.map_err(|failure| format!("fs_next_lines: could not read '{}': {}", reader.path, failure))?;
        if read == 0 {
            at_end = true;
            break;
        }
        while line.ends_with('\n') || line.ends_with('\r') {
            line.pop();
        }
        lines.push(line);
    }

    if at_end {
        drop(open);
        OPEN_READERS.remove(&reader.handle);
    }
    return Ok(lines);
}

/// Closes a reader. A reader that already reached the end is closed, and closing
/// it again is not an error - a program should be able to close what it opened
/// without first working out whether it still needs to.
pub async fn close_reader(reader: &FS_Reader) -> Result<(), String> {
    OPEN_READERS.remove(&reader.handle);
    return Ok(());
}

/// Adds one file to the end of another, copying in blocks so neither has to fit
/// in memory. The target is created if it is not there.
///
/// This is how the pieces of a resumable upload are put back together, and the
/// one thing a program genuinely needs to do to a large binary file that is not
/// already `fs_copy` or `fs_move`.
pub async fn append_from_file(from_path: String, to_path: String) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut source = tokio::fs::File::open(&from_path).await.map_err(|failure| format!("fs_append_file: could not read '{}': {}", from_path, failure))?;
    let mut target = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&to_path)
        .await
        .map_err(|failure| format!("fs_append_file: could not write '{}': {}", to_path, failure))?;

    let mut block = vec![0u8; 64 * 1024];
    loop {
        let read = source.read(&mut block).await.map_err(|failure| format!("fs_append_file: could not read '{}': {}", from_path, failure))?;
        if read == 0 {
            break;
        }
        target.write_all(&block[..read]).await.map_err(|failure| format!("fs_append_file: could not write '{}': {}", to_path, failure))?;
    }
    target.flush().await.map_err(|failure| format!("fs_append_file: could not finish writing '{}': {}", to_path, failure))?;
    return Ok(());
}

/// The most that can be asked for in one range read. A range is for looking at a
/// header or a footer, not for reading a file in disguise.
const LARGEST_RANGE_BYTES: i64 = 8 * 1024 * 1024;

/// Reads exactly the bytes from `offset` for `length`, without reading anything
/// before or after them. Fewer bytes come back if the file ends first.
async fn read_range(path: &str, offset: i64, length: i64, function_name: &str) -> Result<Vec<u8>, String> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    if offset < 0 {
        return Err(format!("{}: the offset cannot be negative, got {}", function_name, offset));
    }
    if length < 1 {
        return Err(format!("{}: asked for {} bytes, which is not a length to read", function_name, length));
    }
    if length > LARGEST_RANGE_BYTES {
        return Err(format!("{}: asked for {} bytes, more than the {} a range read allows - a range is for a header, not for a whole file", function_name, length, LARGEST_RANGE_BYTES));
    }

    let mut file = tokio::fs::File::open(path).await.map_err(|failure| format!("{}: could not read '{}': {}", function_name, path, failure))?;
    file.seek(std::io::SeekFrom::Start(offset as u64)).await.map_err(|failure| format!("{}: could not seek to {} in '{}': {}", function_name, offset, path, failure))?;

    let mut buffer = vec![0u8; length as usize];
    let mut filled = 0usize;
    while filled < buffer.len() {
        let read = file.read(&mut buffer[filled..]).await.map_err(|failure| format!("{}: could not read '{}': {}", function_name, path, failure))?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    buffer.truncate(filled);
    return Ok(buffer);
}

/// A slice of a file as base64, for looking inside one without loading it.
pub async fn read_range_base64(path: String, offset: i64, length: i64) -> Result<String, String> {
    use base64::Engine;
    let bytes = read_range(&path, offset, length, "fs_read_range_base64").await?;
    return Ok(base64::engine::general_purpose::STANDARD.encode(bytes));
}

/// A slice of a file as hex, which is what reading a file's first bytes to work
/// out what it is wants - a PNG starts `89504e47`, a zip `504b0304`.
pub async fn read_range_hex(path: String, offset: i64, length: i64) -> Result<String, String> {
    let bytes = read_range(&path, offset, length, "fs_read_range_hex").await?;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes.iter() {
        out.push_str(&format!("{:02x}", byte));
    }
    return Ok(out);
}

#[cfg(test)]
mod reader_tests {
    use super::*;

    fn a_file(name: &str, contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("nail_reader_{}", name));
        std::fs::write(&path, contents).expect("a writable file");
        return path.to_string_lossy().to_string();
    }

    #[tokio::test]
    async fn lines_come_back_a_piece_at_a_time() {
        let path = a_file("pieces.txt", "one\ntwo\nthree\nfour\nfive\n");
        let reader = open_reader(path.clone()).await.expect("a readable file");

        assert_eq!(next_lines(&reader, 2).await.expect("two lines"), vec!["one".to_string(), "two".to_string()]);
        assert_eq!(next_lines(&reader, 2).await.expect("two more"), vec!["three".to_string(), "four".to_string()]);
        // The last read returns what is left, which is fewer than asked for.
        assert_eq!(next_lines(&reader, 2).await.expect("the rest"), vec!["five".to_string()]);
        assert!(next_lines(&reader, 2).await.expect("nothing left").is_empty());

        close_reader(&reader).await.expect("closing what is already closed");
        let _ = std::fs::remove_file(&path);
    }

    /// The empty answer is what a loop stops on, and by then there is nothing
    /// left open - so a loop that runs to the end leaks nothing.
    #[tokio::test]
    async fn a_reader_closes_itself_at_the_end() {
        let path = a_file("closes.txt", "only line\n");
        let reader = open_reader(path.clone()).await.expect("a readable file");
        assert_eq!(next_lines(&reader, 10).await.expect("the one line").len(), 1);
        assert!(next_lines(&reader, 10).await.expect("nothing left").is_empty());
        assert!(!OPEN_READERS.contains_key(&reader.handle), "the reader should have closed itself");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_file_with_no_final_newline_still_gives_its_last_line() {
        let path = a_file("no_newline.txt", "first\nlast without newline");
        let reader = open_reader(path.clone()).await.expect("a readable file");
        assert_eq!(next_lines(&reader, 10).await.expect("both lines"), vec!["first".to_string(), "last without newline".to_string()]);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn windows_line_endings_are_not_left_on_the_lines() {
        let path = a_file("crlf.txt", "one\r\ntwo\r\n");
        let reader = open_reader(path.clone()).await.expect("a readable file");
        assert_eq!(next_lines(&reader, 10).await.expect("both lines"), vec!["one".to_string(), "two".to_string()]);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn an_empty_file_reads_as_nothing() {
        let path = a_file("empty.txt", "");
        let reader = open_reader(path.clone()).await.expect("a readable file");
        assert!(next_lines(&reader, 10).await.expect("nothing at all").is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn opening_something_that_is_not_there_says_so() {
        assert!(open_reader("/tmp/nail_no_such_file_to_read".to_string()).await.is_err());
    }

    #[tokio::test]
    async fn asking_for_no_lines_is_refused() {
        let path = a_file("refused.txt", "one\n");
        let reader = open_reader(path.clone()).await.expect("a readable file");
        assert!(next_lines(&reader, 0).await.is_err());
        let _ = std::fs::remove_file(&path);
    }

    /// A reader that was never opened - a handle from a previous run, say - reads
    /// as finished rather than failing, which is what a closed reader does too.
    #[tokio::test]
    async fn a_handle_that_is_not_open_reads_as_finished() {
        let unknown = FS_Reader { handle: "fs_reader_nothing".to_string(), path: "gone.txt".to_string() };
        assert!(next_lines(&unknown, 5).await.expect("no lines").is_empty());
    }

    #[tokio::test]
    async fn a_file_is_added_to_the_end_of_another() {
        let first = a_file("part_one.bin", "first half ");
        let second = a_file("part_two.bin", "second half");
        let joined = std::env::temp_dir().join("nail_reader_joined.bin");
        let _ = std::fs::remove_file(&joined);

        append_from_file(first.clone(), joined.to_string_lossy().to_string()).await.expect("a writable target");
        append_from_file(second.clone(), joined.to_string_lossy().to_string()).await.expect("a writable target");
        assert_eq!(std::fs::read_to_string(&joined).expect("the joined file"), "first half second half");

        for path in [first, second] {
            let _ = std::fs::remove_file(path);
        }
        let _ = std::fs::remove_file(&joined);
    }

    #[tokio::test]
    async fn appending_something_that_is_not_there_says_so() {
        let failure = append_from_file("/tmp/nail_no_such_part".to_string(), std::env::temp_dir().join("nail_reader_never.bin").to_string_lossy().to_string()).await.unwrap_err();
        assert!(failure.contains("fs_append_file"), "got: {}", failure);
    }

    #[tokio::test]
    async fn a_slice_of_a_file_is_read_without_the_rest() {
        // A PNG's first bytes, which is what a range read is for.
        let path = std::env::temp_dir().join("nail_reader_range.bin");
        let mut contents: Vec<u8> = vec![0x89, 0x50, 0x4e, 0x47];
        contents.extend_from_slice(&vec![b'x'; 5000]);
        std::fs::write(&path, &contents).expect("a writable file");
        let path = path.to_string_lossy().to_string();

        assert_eq!(read_range_hex(path.clone(), 0, 4).await.expect("the first bytes"), "89504e47");
        assert_eq!(read_range_hex(path.clone(), 4, 2).await.expect("bytes further in"), "7878");
        assert_eq!(read_range_base64(path.clone(), 4, 3).await.expect("bytes as base64"), "eHh4");
        // Past the end gives what there is rather than an error.
        assert_eq!(read_range_hex(path.clone(), 5002, 100).await.expect("the tail").len(), 4);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_range_that_makes_no_sense_is_refused() {
        let path = a_file("range_refused.bin", "abcdef");
        assert!(read_range_hex(path.clone(), -1, 4).await.is_err());
        assert!(read_range_hex(path.clone(), 0, 0).await.is_err());
        let too_much = read_range_base64(path.clone(), 0, 9 * 1024 * 1024).await.unwrap_err();
        assert!(too_much.contains("not for a whole file"), "got: {}", too_much);
        let _ = std::fs::remove_file(&path);
    }
}

/// A running watch on a directory, held by handle like an open reader.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FS_Watcher {
    pub handle: String,
    pub path: String,
}

/// What one watch holds: the OS watcher, which must stay alive for events to
/// keep coming, and the queue they arrive on.
struct RunningWatch {
    _watcher: notify::RecommendedWatcher,
    receiver: std::sync::Mutex<std::sync::mpsc::Receiver<notify::Result<notify::Event>>>,
}

lazy_static::lazy_static! {
    static ref OPEN_WATCHES: dashmap::DashMap<String, std::sync::Arc<RunningWatch>> = dashmap::DashMap::new();
}

/// Starts watching a file or directory (directories are watched all the way
/// down). Changes pile up until `fs_watch_next` collects them, so nothing is
/// missed between calls; `fs_watch_stop` ends the watch.
///
/// What a watch is for: a dev server rebuilding when a source file changes, a
/// job picking up files dropped into a directory. The operating system does the
/// watching, so waiting costs nothing.
pub async fn watch_start(path: String) -> Result<FS_Watcher, String> {
    use notify::Watcher;

    if !Path::new(&path).exists() {
        return Err(format!("fs_watch_start: there is nothing at '{}' to watch", path));
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(sender).map_err(|failure| format!("fs_watch_start: could not start watching '{}': {}", path, failure))?;
    watcher.watch(Path::new(&path), notify::RecursiveMode::Recursive).map_err(|failure| format!("fs_watch_start: could not start watching '{}': {}", path, failure))?;

    let handle = format!("fs_watch_{}", uuid::Uuid::new_v4());
    OPEN_WATCHES.insert(handle.clone(), std::sync::Arc::new(RunningWatch { _watcher: watcher, receiver: std::sync::Mutex::new(receiver) }));
    return Ok(FS_Watcher { handle, path });
}

/// The paths that changed since the last call, waiting up to the timeout for
/// the first change. An empty array means the time passed quietly. Each path
/// appears once however many events it produced, sorted.
pub async fn watch_next(watcher: &FS_Watcher, timeout_milliseconds: i64) -> Result<Vec<String>, String> {
    if timeout_milliseconds < 0 {
        return Err(format!("fs_watch_next: the timeout cannot be negative, got {}", timeout_milliseconds));
    }
    let watch = OPEN_WATCHES.get(&watcher.handle).map(|entry| entry.value().clone()).ok_or_else(|| format!("fs_watch_next: unknown watch handle '{}' (was it already stopped?)", watcher.handle))?;

    let wait = std::time::Duration::from_millis(timeout_milliseconds as u64);
    let changed = tokio::task::spawn_blocking(move || {
        let receiver = watch.receiver.lock().map_err(|_| "fs_watch_next: the watch broke and cannot be read".to_string())?;
        let mut paths: Vec<String> = Vec::new();
        // Block for the first event only; after one arrives, drain whatever
        // else is already queued so one save does not become five answers.
        match receiver.recv_timeout(wait) {
            Ok(Ok(event)) => paths.extend(event.paths.iter().map(|path| path.to_string_lossy().to_string())),
            Ok(Err(failure)) => return Err(format!("fs_watch_next: the watch reported an error: {}", failure)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return Ok(Vec::new()),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Err("fs_watch_next: the watch has stopped".to_string()),
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        while let Ok(Ok(event)) = receiver.try_recv() {
            paths.extend(event.paths.iter().map(|path| path.to_string_lossy().to_string()));
        }
        paths.sort();
        paths.dedup();
        return Ok(paths);
    })
    .await
    .map_err(|failure| format!("fs_watch_next: the watch task failed: {}", failure))??;
    return Ok(changed);
}

/// Ends a watch and forgets its handle. Stopping one twice is not an error.
pub async fn watch_stop(watcher: &FS_Watcher) -> Result<(), String> {
    OPEN_WATCHES.remove(&watcher.handle);
    return Ok(());
}

#[cfg(test)]
mod watch_tests {
    use super::*;

    fn a_directory(name: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!("nail_watch_{}", name));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a writable temporary directory");
        return directory;
    }

    #[tokio::test]
    async fn a_change_is_seen_and_quiet_time_is_empty() {
        let directory = a_directory("sees_changes");
        let watcher = watch_start(directory.to_string_lossy().to_string()).await.expect("a watchable directory");

        std::fs::write(directory.join("dropped.txt"), "arrived").expect("a writable file");
        let changed = watch_next(&watcher, 5000).await.expect("a running watch");
        assert!(changed.iter().any(|path| path.ends_with("dropped.txt")), "got: {:?}", changed);

        // Nothing else happens, so the next call times out quietly.
        let quiet = watch_next(&watcher, 100).await.expect("a running watch");
        assert!(quiet.is_empty(), "got: {:?}", quiet);

        watch_stop(&watcher).await.expect("stopping");
        watch_stop(&watcher).await.expect("stopping twice is fine");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[tokio::test]
    async fn a_stopped_watch_says_so() {
        let directory = a_directory("stopped");
        let watcher = watch_start(directory.to_string_lossy().to_string()).await.expect("a watchable directory");
        watch_stop(&watcher).await.expect("stopping");
        let failure = watch_next(&watcher, 100).await.unwrap_err();
        assert!(failure.contains("unknown watch handle"), "got: {}", failure);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[tokio::test]
    async fn watching_nothing_says_so() {
        assert!(watch_start("/tmp/nail_no_such_directory_to_watch".to_string()).await.is_err());
    }
}
