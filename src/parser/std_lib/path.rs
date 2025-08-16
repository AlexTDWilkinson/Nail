use std::path::{Path, PathBuf};

pub async fn join(base: String, path: String) -> String {
    Path::new(&base).join(&path).to_string_lossy().to_string()
}

pub async fn exists(path: String) -> bool {
    Path::new(&path).exists()
}

/// Get the filename from a path
pub async fn basename(path: String) -> String {
    Path::new(&path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// Get the directory from a path
pub async fn dirname(path: String) -> String {
    Path::new(&path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_string()
}

/// Get the file extension from a path
pub async fn extension(path: String) -> Result<String, String> {
    Path::new(&path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("No extension found for path: {}", path))
}

/// Check if a path is absolute
pub async fn is_absolute(path: String) -> bool {
    Path::new(&path).is_absolute()
}

/// Normalize a path (resolve . and ..)
pub async fn normalize(path: String) -> String {
    let path = Path::new(&path);
    let mut components = Vec::new();
    
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                components.pop();
            }
            Component::CurDir => {
                // Skip current directory markers
            }
            c => {
                components.push(c.as_os_str().to_string_lossy().to_string());
            }
        }
    }
    
    if components.is_empty() {
        ".".to_string()
    } else {
        components.join("/")
    }
}