//! Mime module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Mime:
        "mime_for_path" => "std_lib::mime::for_path", (path: (&s)) -> s,
            "What a browser should be told a file is, worked out from its name. An unknown or missing extension gives application/octet-stream rather than a guess.",
            "request_path:s = `/static/style.css`;\ncontent_type:s = mime_for_path(request_path);";
        "mime_is_text" => "std_lib::mime::is_text", (media_type: (&s)) -> b,
            "Whether a media type is text a program could read as a string, which covers text/* and the structured formats spelled application/json and the like.",
            "content_type:s = mime_for_path(`/static/style.css`);\nreadable:b = mime_is_text(content_type);";
        "mime_extension_for" => "std_lib::mime::extension_for", (media_type: (&s)) -> (s!e),
            "The usual extension for a media type, without the dot, for naming a file that arrived with a type but no name.",
            "extension:s = danger(mime_extension_for(`image/png`));";
    }
}
