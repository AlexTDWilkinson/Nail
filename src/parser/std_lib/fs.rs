use std::path::Path;

pub async fn read_file(path: String) -> Result<String, String> {
    tokio::fs::read_to_string(Path::new(&path))
        .await
        .map_err(|e| format!("Failed to read file '{}': {}", path, e))
}

pub async fn write_file(path: String, content: String) -> Result<(), String> {
    tokio::fs::write(Path::new(&path), content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", path, e))
}

pub async fn create_dir(path: String) -> Result<(), String> {
    tokio::fs::create_dir_all(Path::new(&path))
        .await
        .map_err(|e| format!("Failed to create directory '{}': {}", path, e))
}

pub async fn remove_file(path: String) -> Result<(), String> {
    tokio::fs::remove_file(Path::new(&path))
        .await
        .map_err(|e| format!("Failed to remove file '{}': {}", path, e))
}

pub async fn copy(from: String, to: String) -> Result<(), String> {
    tokio::fs::copy(Path::new(&from), Path::new(&to))
        .await
        .map(|_| ())
        .map_err(|e| format!("Failed to copy '{}' to '{}': {}", from, to, e))
}

pub async fn move_file(from: String, to: String) -> Result<(), String> {
    tokio::fs::rename(Path::new(&from), Path::new(&to))
        .await
        .map_err(|e| format!("Failed to move '{}' to '{}': {}", from, to, e))
}