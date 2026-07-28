use std::path::{Path, PathBuf};

pub fn join(base: String, path: String) -> String {
    Path::new(&base).join(&path).to_string_lossy().to_string()
}

pub fn exists(path: String) -> bool {
    Path::new(&path).exists()
}

/// Get the filename from a path
pub fn basename(path: String) -> String {
    Path::new(&path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// Get the directory from a path
pub fn dirname(path: String) -> String {
    Path::new(&path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_string()
}

/// Get the file extension from a path
pub fn extension(path: String) -> Result<String, String> {
    Path::new(&path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("path_extension: no file extension in '{}'", path))
}

/// Check if a path is absolute
pub fn is_absolute(path: String) -> bool {
    Path::new(&path).is_absolute()
}

/// Normalize a path (resolve . and ..)
pub fn normalize(path: String) -> String {
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