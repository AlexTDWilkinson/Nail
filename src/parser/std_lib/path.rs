use std::path::Path;

pub fn join(base: String, path: String) -> String {
    Path::new(&base).join(&path).to_string_lossy().to_string()
}

pub fn exists(path: String) -> bool {
    Path::new(&path).exists()
}