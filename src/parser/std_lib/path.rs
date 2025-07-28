use std::path::Path;

pub async fn join(base: String, path: String) -> String {
    Path::new(&base).join(&path).to_string_lossy().to_string()
}

pub async fn exists(path: String) -> bool {
    Path::new(&path).exists()
}