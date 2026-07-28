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