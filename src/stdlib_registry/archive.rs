//! Archive module stdlib registry entries.
//!
//! Path to path throughout: a directory in, an archive file out, and back.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Archive:
        "archive_zip_create" [Zip, Tokio] => "std_lib::archive::zip_create", (zip_path: s, directory: s) -> (v!e),
            "Writes every file at or below the directory into one compressed zip file.",
            "danger(fs_create_dir(`reports`));\ndanger(fs_write(`reports/summary.txt`, `all good`));\ndanger(archive_zip_create(`backup.zip`, `reports`));";
        "archive_zip_extract" [Zip, Tokio] => "std_lib::archive::zip_extract", (zip_path: s, directory: s) -> (v!e),
            "Unpacks a zip file into a directory, creating it if needed. An entry naming a path outside that directory is refused rather than written.",
            "danger(fs_create_dir(`reports`));\ndanger(fs_write(`reports/summary.txt`, `all good`));\ndanger(archive_zip_create(`download.zip`, `reports`));\ndanger(archive_zip_extract(`download.zip`, `unpacked`));";
        "archive_zip_list" [Zip, Tokio] => "std_lib::archive::zip_list", (zip_path: s) -> ([s]!e),
            "Returns the paths inside a zip file without unpacking it.",
            "danger(fs_create_dir(`reports`));\ndanger(fs_write(`reports/summary.txt`, `all good`));\ndanger(archive_zip_create(`download.zip`, `reports`));\ncontents:a:s = danger(archive_zip_list(`download.zip`));";
        "archive_targz_create" [Tar, Flate2, Tokio] => "std_lib::archive::targz_create", (archive_path: s, directory: s) -> (v!e),
            "Writes every file at or below the directory into one gzipped tar file.",
            "danger(fs_create_dir(`reports`));\ndanger(fs_write(`reports/summary.txt`, `all good`));\ndanger(archive_targz_create(`backup.tar.gz`, `reports`));";
        "archive_targz_extract" [Tar, Flate2, Tokio] => "std_lib::archive::targz_extract", (archive_path: s, directory: s) -> (v!e),
            "Unpacks a gzipped tar file into a directory. An entry naming a path outside that directory is refused, and links and devices are skipped.",
            "danger(fs_create_dir(`reports`));\ndanger(fs_write(`reports/summary.txt`, `all good`));\ndanger(archive_targz_create(`release.tar.gz`, `reports`));\ndanger(archive_targz_extract(`release.tar.gz`, `unpacked`));";
        "archive_targz_list" [Tar, Flate2, Tokio] => "std_lib::archive::targz_list", (archive_path: s) -> ([s]!e),
            "Returns the paths inside a gzipped tar file without unpacking it.",
            "danger(fs_create_dir(`reports`));\ndanger(fs_write(`reports/summary.txt`, `all good`));\ndanger(archive_targz_create(`release.tar.gz`, `reports`));\ncontents:a:s = danger(archive_targz_list(`release.tar.gz`));";
        "archive_tarzst_create" [Tar, Zstd, Tokio] => "std_lib::archive::tarzst_create", (archive_path: s, directory: s) -> (v!e),
            "Writes every file at or below the directory into one zstd compressed tar file.",
            "danger(fs_create_dir(`reports`));\ndanger(fs_write(`reports/summary.txt`, `all good`));\ndanger(archive_tarzst_create(`backup.tar.zst`, `reports`));";
        "archive_tarzst_extract" [Tar, Zstd, Tokio] => "std_lib::archive::tarzst_extract", (archive_path: s, directory: s) -> (v!e),
            "Unpacks a zstd compressed tar file into a directory. An entry naming a path outside that directory is refused, and links and devices are skipped.",
            "danger(fs_create_dir(`reports`));\ndanger(fs_write(`reports/summary.txt`, `all good`));\ndanger(archive_tarzst_create(`release.tar.zst`, `reports`));\ndanger(archive_tarzst_extract(`release.tar.zst`, `unpacked`));";
        "archive_tarzst_list" [Tar, Zstd, Tokio] => "std_lib::archive::tarzst_list", (archive_path: s) -> ([s]!e),
            "Returns the paths inside a zstd compressed tar file without unpacking it.",
            "danger(fs_create_dir(`reports`));\ndanger(fs_write(`reports/summary.txt`, `all good`));\ndanger(archive_tarzst_create(`release.tar.zst`, `reports`));\ncontents:a:s = danger(archive_tarzst_list(`release.tar.zst`));";
    }
}
