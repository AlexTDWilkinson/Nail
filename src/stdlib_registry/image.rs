//! Image module stdlib registry entries.
//!
//! File to file throughout: read that path, write this one. Nothing binary
//! crosses into Nail.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Image:
        "image_resize" [Image, Tokio] => "std_lib::image::resize", (from_path: s, to_path: s, width: i, height: i) -> (v!e),
            "Writes a copy of the picture at exactly that size. The written path's extension decides the format, so this converts as well as resizes.",
            "danger(image_resize(`upload.png`, `thumb.jpg`, 200, 200));";
        "image_resize_within" [Image, Tokio] => "std_lib::image::resize_within", (from_path: s, to_path: s, width: i, height: i) -> (v!e),
            "Writes a copy that fits inside the given box without stretching, so one side comes out smaller than asked for. A picture already smaller is copied at its own size.",
            "danger(image_resize_within(`upload.png`, `thumb.png`, 200, 200));";
        "image_convert" [Image, Tokio] => "std_lib::image::convert", (from_path: s, to_path: s) -> (v!e),
            "Writes the picture in whatever format the written path's extension names.",
            "danger(image_convert(`photo.png`, `photo.webp`));";
        "image_width" [Image, Tokio] => "std_lib::image::width", (path: s) -> (i!e),
            "How many pixels wide the picture is.",
            "wide:i = danger(image_width(`upload.png`));";
        "image_height" [Image, Tokio] => "std_lib::image::height", (path: s) -> (i!e),
            "How many pixels tall the picture is.",
            "tall:i = danger(image_height(`upload.png`));";
        "image_format" [Image, Tokio] => "std_lib::image::format", (path: s) -> (s!e),
            "What format the file actually is, read from its bytes rather than its name - the check worth doing on an upload before storing it.",
            "kind:s = danger(image_format(request.body_path));";
    }
}
