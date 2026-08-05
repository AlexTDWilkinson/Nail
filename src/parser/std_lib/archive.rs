//! Archives: a directory in one file, and back out again.
//!
//! Everything here works from path to path - a directory in, an archive file
//! out - rather than through a value in memory. That is deliberate: an archive
//! of anything worth archiving does not fit comfortably in memory, and the
//! interesting operations (back this directory up, unpack what was downloaded)
//! are both about files on disk anyway.
//!
//! Zip is here because it is what the rest of the world sends; tar.gz is here
//! because it is what Unix sends. `compress_gzip` already handles a single
//! string, so this module is for the case of many files.

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

/// Refuses a path inside an archive that would write outside the directory
/// being extracted into. An archive is data from somewhere else, and a member
/// named `../../etc/cron.d/anything` is a well-known way to turn unpacking a
/// download into running a program. Only the plain-name components survive.
fn safe_member_path(root: &Path, member: &str) -> Result<PathBuf, String> {
    let mut out = root.to_path_buf();
    let mut kept_anything = false;
    for component in Path::new(member).components() {
        match component {
            Component::Normal(part) => {
                out.push(part);
                kept_anything = true;
            }
            // A parent, a root or a drive letter is what the attack looks like.
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("the archive holds an entry named `{}`, which points outside the directory being extracted into", member));
            }
            Component::CurDir => {}
        }
    }
    if !kept_anything {
        return Err(format!("the archive holds an entry named `{}`, which names no file", member));
    }
    return Ok(out);
}

/// Every file at or below a directory, with the path each should have inside an
/// archive - the path relative to the directory itself, so unpacking gives back
/// the same shape rather than a copy of the machine's directory tree.
fn files_to_archive(directory: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let mut found: Vec<(PathBuf, String)> = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = std::fs::read_dir(&current).map_err(|failure| format!("could not read directory '{}': {}", current.display(), failure))?;
        for entry in entries {
            let entry = entry.map_err(|failure| format!("could not read directory '{}': {}", current.display(), failure))?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|failure| format!("could not inspect '{}': {}", path.display(), failure))?;
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                let relative = path.strip_prefix(directory).map_err(|_| format!("'{}' is not inside '{}'", path.display(), directory.display()))?;
                // Zip and tar both use forward slashes whatever the machine does.
                let inside = relative.components().filter_map(|component| component.as_os_str().to_str()).collect::<Vec<&str>>().join("/");
                found.push((path, inside));
            }
        }
    }
    found.sort_by(|left, right| left.1.cmp(&right.1));
    return Ok(found);
}

fn directory_must_exist(directory: &str, function_name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(directory);
    if !path.is_dir() {
        return Err(format!("{}: '{}' is not a directory", function_name, directory));
    }
    return Ok(path);
}

/// Writes every file at or below a directory into one zip file, compressed.
/// Empty directories are not kept, because a zip is a list of files.
pub async fn zip_create(zip_path: String, directory: String) -> Result<(), String> {
    let root = directory_must_exist(&directory, "archive_zip_create")?;
    let members = files_to_archive(&root).map_err(|detail| format!("archive_zip_create: {}", detail))?;

    let file = File::create(&zip_path).map_err(|failure| format!("archive_zip_create: could not write '{}': {}", zip_path, failure))?;
    let mut writer = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for (path, inside) in members {
        writer.start_file(inside.clone(), options).map_err(|failure| format!("archive_zip_create: could not add '{}': {}", inside, failure))?;
        let mut contents = Vec::new();
        File::open(&path)
            .and_then(|mut source| source.read_to_end(&mut contents))
            .map_err(|failure| format!("archive_zip_create: could not read '{}': {}", path.display(), failure))?;
        writer.write_all(&contents).map_err(|failure| format!("archive_zip_create: could not add '{}': {}", inside, failure))?;
    }
    writer.finish().map_err(|failure| format!("archive_zip_create: could not finish '{}': {}", zip_path, failure))?;
    return Ok(());
}

/// Unpacks a zip file into a directory, creating it if it is not there. An
/// entry naming a path outside that directory stops the extraction rather than
/// being written.
pub async fn zip_extract(zip_path: String, directory: String) -> Result<(), String> {
    let file = File::open(&zip_path).map_err(|failure| format!("archive_zip_extract: could not read '{}': {}", zip_path, failure))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|failure| format!("archive_zip_extract: '{}' is not a zip file: {}", zip_path, failure))?;
    let root = PathBuf::from(&directory);
    std::fs::create_dir_all(&root).map_err(|failure| format!("archive_zip_extract: could not create '{}': {}", directory, failure))?;

    for index in 0..archive.len() {
        let mut member = archive.by_index(index).map_err(|failure| format!("archive_zip_extract: could not read entry {} of '{}': {}", index, zip_path, failure))?;
        let name = member.name().to_string();
        if name.ends_with('/') {
            let target = safe_member_path(&root, &name).map_err(|detail| format!("archive_zip_extract: {}", detail))?;
            std::fs::create_dir_all(&target).map_err(|failure| format!("archive_zip_extract: could not create '{}': {}", target.display(), failure))?;
            continue;
        }
        let target = safe_member_path(&root, &name).map_err(|detail| format!("archive_zip_extract: {}", detail))?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|failure| format!("archive_zip_extract: could not create '{}': {}", parent.display(), failure))?;
        }
        let mut contents = Vec::new();
        member.read_to_end(&mut contents).map_err(|failure| format!("archive_zip_extract: could not read '{}': {}", name, failure))?;
        std::fs::write(&target, contents).map_err(|failure| format!("archive_zip_extract: could not write '{}': {}", target.display(), failure))?;
    }
    return Ok(());
}

/// The paths inside a zip file, without unpacking it.
pub async fn zip_list(zip_path: String) -> Result<Vec<String>, String> {
    let file = File::open(&zip_path).map_err(|failure| format!("archive_zip_list: could not read '{}': {}", zip_path, failure))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|failure| format!("archive_zip_list: '{}' is not a zip file: {}", zip_path, failure))?;
    let mut names = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let member = archive.by_index(index).map_err(|failure| format!("archive_zip_list: could not read entry {} of '{}': {}", index, zip_path, failure))?;
        names.push(member.name().to_string());
    }
    names.sort();
    return Ok(names);
}

/// Writes every file at or below a directory into one gzipped tar file - the
/// `.tar.gz` everything on Unix arrives as.
pub async fn targz_create(archive_path: String, directory: String) -> Result<(), String> {
    let root = directory_must_exist(&directory, "archive_targz_create")?;
    let file = File::create(&archive_path).map_err(|failure| format!("archive_targz_create: could not write '{}': {}", archive_path, failure))?;
    let mut builder = tar::Builder::new(GzEncoder::new(file, Compression::default()));

    for (path, inside) in files_to_archive(&root).map_err(|detail| format!("archive_targz_create: {}", detail))? {
        let mut source = File::open(&path).map_err(|failure| format!("archive_targz_create: could not read '{}': {}", path.display(), failure))?;
        builder.append_file(&inside, &mut source).map_err(|failure| format!("archive_targz_create: could not add '{}': {}", inside, failure))?;
    }
    builder
        .into_inner()
        .and_then(|encoder| encoder.finish())
        .map_err(|failure| format!("archive_targz_create: could not finish '{}': {}", archive_path, failure))?;
    return Ok(());
}

/// Unpacks a gzipped tar file into a directory. As with zip, an entry naming a
/// path outside that directory stops the extraction.
pub async fn targz_extract(archive_path: String, directory: String) -> Result<(), String> {
    let file = File::open(&archive_path).map_err(|failure| format!("archive_targz_extract: could not read '{}': {}", archive_path, failure))?;
    let root = PathBuf::from(&directory);
    std::fs::create_dir_all(&root).map_err(|failure| format!("archive_targz_extract: could not create '{}': {}", directory, failure))?;

    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let entries = archive.entries().map_err(|failure| format!("archive_targz_extract: '{}' is not a gzipped tar file: {}", archive_path, failure))?;
    for entry in entries {
        let mut entry = entry.map_err(|failure| format!("archive_targz_extract: could not read an entry of '{}': {}", archive_path, failure))?;
        let name = entry.path().map_err(|failure| format!("archive_targz_extract: an entry of '{}' has an unreadable name: {}", archive_path, failure))?.to_string_lossy().to_string();
        if entry.header().entry_type().is_dir() {
            let target = safe_member_path(&root, &name).map_err(|detail| format!("archive_targz_extract: {}", detail))?;
            std::fs::create_dir_all(&target).map_err(|failure| format!("archive_targz_extract: could not create '{}': {}", target.display(), failure))?;
            continue;
        }
        if !entry.header().entry_type().is_file() {
            // Links and devices are skipped rather than recreated: a symlink out
            // of the directory is the same attack as a `..` in a path.
            continue;
        }
        let target = safe_member_path(&root, &name).map_err(|detail| format!("archive_targz_extract: {}", detail))?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|failure| format!("archive_targz_extract: could not create '{}': {}", parent.display(), failure))?;
        }
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents).map_err(|failure| format!("archive_targz_extract: could not read '{}': {}", name, failure))?;
        std::fs::write(&target, contents).map_err(|failure| format!("archive_targz_extract: could not write '{}': {}", target.display(), failure))?;
    }
    return Ok(());
}

/// The paths inside a gzipped tar file, without unpacking it.
pub async fn targz_list(archive_path: String) -> Result<Vec<String>, String> {
    let file = File::open(&archive_path).map_err(|failure| format!("archive_targz_list: could not read '{}': {}", archive_path, failure))?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let entries = archive.entries().map_err(|failure| format!("archive_targz_list: '{}' is not a gzipped tar file: {}", archive_path, failure))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|failure| format!("archive_targz_list: could not read an entry of '{}': {}", archive_path, failure))?;
        let name = entry.path().map_err(|failure| format!("archive_targz_list: an entry of '{}' has an unreadable name: {}", archive_path, failure))?;
        names.push(name.to_string_lossy().to_string());
    }
    names.sort();
    return Ok(names);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory of files to archive, under a name of its own so tests do not
    /// tread on each other.
    fn a_directory_with_files(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("nail_archive_{}", name));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("nested")).expect("a writable temporary directory");
        std::fs::write(root.join("top.txt"), "the top file").expect("a writable file");
        std::fs::write(root.join("nested").join("under.txt"), "the nested file").expect("a writable file");
        return root;
    }

    #[tokio::test]
    async fn a_zip_round_trips_a_directory() {
        let source = a_directory_with_files("zip_round_trip");
        let archive = std::env::temp_dir().join("nail_archive_zip_round_trip.zip");
        let destination = std::env::temp_dir().join("nail_archive_zip_round_trip_out");
        let _ = std::fs::remove_file(&archive);
        let _ = std::fs::remove_dir_all(&destination);

        zip_create(archive.to_string_lossy().to_string(), source.to_string_lossy().to_string()).await.expect("a writable archive");
        let listed = zip_list(archive.to_string_lossy().to_string()).await.expect("a readable archive");
        assert_eq!(listed, vec!["nested/under.txt".to_string(), "top.txt".to_string()]);

        zip_extract(archive.to_string_lossy().to_string(), destination.to_string_lossy().to_string()).await.expect("an extractable archive");
        assert_eq!(std::fs::read_to_string(destination.join("top.txt")).expect("the extracted file"), "the top file");
        assert_eq!(std::fs::read_to_string(destination.join("nested").join("under.txt")).expect("the extracted file"), "the nested file");

        let _ = std::fs::remove_dir_all(&source);
        let _ = std::fs::remove_file(&archive);
        let _ = std::fs::remove_dir_all(&destination);
    }

    #[tokio::test]
    async fn a_targz_round_trips_a_directory() {
        let source = a_directory_with_files("targz_round_trip");
        let archive = std::env::temp_dir().join("nail_archive_targz_round_trip.tar.gz");
        let destination = std::env::temp_dir().join("nail_archive_targz_round_trip_out");
        let _ = std::fs::remove_file(&archive);
        let _ = std::fs::remove_dir_all(&destination);

        targz_create(archive.to_string_lossy().to_string(), source.to_string_lossy().to_string()).await.expect("a writable archive");
        let listed = targz_list(archive.to_string_lossy().to_string()).await.expect("a readable archive");
        assert_eq!(listed, vec!["nested/under.txt".to_string(), "top.txt".to_string()]);

        targz_extract(archive.to_string_lossy().to_string(), destination.to_string_lossy().to_string()).await.expect("an extractable archive");
        assert_eq!(std::fs::read_to_string(destination.join("nested").join("under.txt")).expect("the extracted file"), "the nested file");

        let _ = std::fs::remove_dir_all(&source);
        let _ = std::fs::remove_file(&archive);
        let _ = std::fs::remove_dir_all(&destination);
    }

    /// The whole reason `safe_member_path` exists: unpacking a download must not
    /// be able to write anywhere but the directory it was told to.
    #[test]
    fn an_entry_pointing_outside_the_directory_is_refused() {
        let root = Path::new("/tmp/extract-here");
        assert!(safe_member_path(root, "../../etc/cron.d/anything").is_err());
        assert!(safe_member_path(root, "/etc/passwd").is_err());
        assert!(safe_member_path(root, "..").is_err());
        assert!(safe_member_path(root, "").is_err());

        let inside = safe_member_path(root, "nested/under.txt").expect("a plain path");
        assert_eq!(inside, root.join("nested").join("under.txt"));
        // A leading `./` is harmless and is simply dropped.
        assert_eq!(safe_member_path(root, "./top.txt").expect("a plain path"), root.join("top.txt"));
    }

    #[tokio::test]
    async fn archiving_something_that_is_not_a_directory_says_so() {
        let failure = zip_create("/tmp/nail_archive_never_written.zip".to_string(), "/tmp/nail_archive_no_such_directory".to_string()).await.unwrap_err();
        assert!(failure.contains("is not a directory"), "got: {}", failure);
    }

    #[tokio::test]
    async fn reading_something_that_is_not_an_archive_says_so() {
        let not_an_archive = std::env::temp_dir().join("nail_archive_not_an_archive.zip");
        std::fs::write(&not_an_archive, "this is just text").expect("a writable file");
        let failure = zip_list(not_an_archive.to_string_lossy().to_string()).await.unwrap_err();
        assert!(failure.contains("is not a zip file"), "got: {}", failure);
        let _ = std::fs::remove_file(&not_an_archive);
    }
}
