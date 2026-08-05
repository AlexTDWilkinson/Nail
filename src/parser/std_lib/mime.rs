//! What kind of file this is, by its name.
//!
//! A server has to tell a browser what it is sending, and a browser that is not
//! told guesses - which is how a stylesheet arrives as plain text and is
//! ignored, or worse, how an upload someone named `photo.png` is treated as
//! whatever its bytes look like. `http_server` sets this for the files it
//! serves; a program building a response itself needs it too.
//!
//! The table is the types that actually turn up on the web, not the several
//! thousand IANA has registered. Anything not in it is
//! `application/octet-stream`, which means "bytes, do not try to interpret
//! this" - the safe answer, since guessing is the thing being avoided.

/// Extension to media type. Kept in one place, sorted by what it is, so adding
/// a type is one line.
const TYPES: &[(&str, &str)] = &[
    // Text and markup
    ("html", "text/html; charset=utf-8"),
    ("htm", "text/html; charset=utf-8"),
    ("css", "text/css; charset=utf-8"),
    ("js", "text/javascript; charset=utf-8"),
    ("mjs", "text/javascript; charset=utf-8"),
    ("txt", "text/plain; charset=utf-8"),
    ("md", "text/markdown; charset=utf-8"),
    ("csv", "text/csv; charset=utf-8"),
    ("nail", "text/plain; charset=utf-8"),
    // Structured data
    ("json", "application/json"),
    ("jsonl", "application/jsonl"),
    ("xml", "application/xml"),
    ("yaml", "application/yaml"),
    ("yml", "application/yaml"),
    ("toml", "application/toml"),
    ("wasm", "application/wasm"),
    ("pdf", "application/pdf"),
    ("rss", "application/rss+xml"),
    ("atom", "application/atom+xml"),
    // Images
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("svg", "image/svg+xml"),
    ("webp", "image/webp"),
    ("avif", "image/avif"),
    ("ico", "image/vnd.microsoft.icon"),
    ("bmp", "image/bmp"),
    ("tiff", "image/tiff"),
    // Fonts
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
    ("ttf", "font/ttf"),
    ("otf", "font/otf"),
    // Sound and video
    ("mp3", "audio/mpeg"),
    ("wav", "audio/wav"),
    ("ogg", "audio/ogg"),
    ("flac", "audio/flac"),
    ("m4a", "audio/mp4"),
    ("mp4", "video/mp4"),
    ("webm", "video/webm"),
    ("mov", "video/quicktime"),
    // Archives
    ("zip", "application/zip"),
    ("gz", "application/gzip"),
    ("tar", "application/x-tar"),
    ("bz2", "application/x-bzip2"),
    ("zst", "application/zstd"),
];

/// What a browser should be told a file is, worked out from its name. A name
/// with no extension, or one that is not known, gives
/// `application/octet-stream`.
pub fn for_path(path: &String) -> String {
    let extension = match path.rsplit_once('.') {
        // A dot in a directory name is not an extension: `./archive/file` has no
        // extension at all.
        Some((_, extension)) if !extension.contains('/') && !extension.is_empty() => extension.to_lowercase(),
        _ => return "application/octet-stream".to_string(),
    };
    return match TYPES.iter().find(|(known, _)| *known == extension) {
        Some((_, media_type)) => media_type.to_string(),
        None => "application/octet-stream".to_string(),
    };
}

/// Whether a media type is text a program could read as a string. Everything
/// under `text/` is, and so are the structured formats that happen to be
/// spelled `application/...` - JSON, XML, YAML and the rest.
pub fn is_text(media_type: &String) -> bool {
    let lowered = media_type.to_lowercase();
    if lowered.starts_with("text/") {
        return true;
    }
    const TEXT_APPLICATIONS: [&str; 8] = ["application/json", "application/jsonl", "application/xml", "application/yaml", "application/toml", "application/rss+xml", "application/atom+xml", "application/javascript"];
    return TEXT_APPLICATIONS.iter().any(|known| lowered.starts_with(known)) || lowered.ends_with("+json") || lowered.ends_with("+xml");
}

/// The usual extension for a media type, without the dot - for naming a file
/// that arrived with a type but no name. An unknown type is an error, since
/// inventing an extension would be a guess about the contents.
pub fn extension_for(media_type: &String) -> Result<String, String> {
    // The type may arrive with parameters attached, `text/html; charset=utf-8`,
    // and it is the type itself that is being looked up.
    let bare = media_type.split(';').next().unwrap_or("").trim().to_lowercase();
    for (extension, known) in TYPES.iter() {
        let known_bare = known.split(';').next().unwrap_or("").trim();
        if known_bare == bare {
            return Ok(extension.to_string());
        }
    }
    return Err(format!("mime_extension_for: `{}` is not a media type this knows an extension for", media_type));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(path: &str) -> String {
        return for_path(&path.to_string());
    }

    #[test]
    fn a_name_gives_the_type_a_browser_needs() {
        assert_eq!(of("index.html"), "text/html; charset=utf-8");
        assert_eq!(of("site.css"), "text/css; charset=utf-8");
        assert_eq!(of("app.js"), "text/javascript; charset=utf-8");
        assert_eq!(of("data.json"), "application/json");
        assert_eq!(of("logo.svg"), "image/svg+xml");
        assert_eq!(of("photo.jpeg"), "image/jpeg");
        assert_eq!(of("font.woff2"), "font/woff2");
    }

    #[test]
    fn the_extension_is_read_whatever_case_it_is_in() {
        assert_eq!(of("PHOTO.PNG"), "image/png");
        assert_eq!(of("Index.HtmL"), "text/html; charset=utf-8");
    }

    #[test]
    fn a_path_is_read_as_well_as_a_name() {
        assert_eq!(of("/srv/app/public/index.html"), "text/html; charset=utf-8");
        assert_eq!(of("./images/one.png"), "image/png");
    }

    /// The safe answer for anything unrecognised: bytes, do not interpret.
    #[test]
    fn something_unknown_is_bytes_rather_than_a_guess() {
        assert_eq!(of("archive.unknownext"), "application/octet-stream");
        assert_eq!(of("Makefile"), "application/octet-stream");
        assert_eq!(of(""), "application/octet-stream");
        assert_eq!(of("trailing."), "application/octet-stream");
    }

    /// A dot in a directory name is not the file's extension.
    #[test]
    fn a_dot_in_a_directory_is_not_an_extension() {
        assert_eq!(of("/srv/app.v2/README"), "application/octet-stream");
    }

    #[test]
    fn text_types_are_told_from_binary_ones() {
        assert!(is_text(&"text/html; charset=utf-8".to_string()));
        assert!(is_text(&"application/json".to_string()));
        assert!(is_text(&"application/rss+xml".to_string()));
        assert!(is_text(&"application/vnd.api+json".to_string()));
        assert!(!is_text(&"image/png".to_string()));
        assert!(!is_text(&"application/octet-stream".to_string()));
        assert!(!is_text(&"application/zip".to_string()));
    }

    #[test]
    fn a_type_gives_back_an_extension() {
        assert_eq!(extension_for(&"image/png".to_string()).expect("a known type"), "png");
        // Parameters on the type do not stop it being recognised.
        assert_eq!(extension_for(&"text/html; charset=utf-8".to_string()).expect("a known type"), "html");
        assert_eq!(extension_for(&"text/html".to_string()).expect("a known type"), "html");
        assert!(extension_for(&"application/octet-stream".to_string()).is_err());
        assert!(extension_for(&"nonsense".to_string()).is_err());
    }

    /// Every type in the table can be looked up in both directions, so no entry
    /// is reachable only one way.
    #[test]
    fn the_table_works_in_both_directions() {
        for (extension, media_type) in TYPES.iter() {
            assert_eq!(&for_path(&format!("file.{}", extension)), media_type);
            assert!(extension_for(&media_type.to_string()).is_ok(), "no extension for {}", media_type);
        }
    }
}
