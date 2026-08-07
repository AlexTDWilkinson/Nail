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
        "image_thumbnail" [Image, Tokio] => "std_lib::image::thumbnail", (from_path: s, to_path: s, size: i) -> (v!e),
            "Writes a square thumbnail filled edge to edge: the picture is scaled until it covers the square and the overhanging sides are cut off evenly, so a grid of these lines up whatever shape the pictures were.",
            "danger(image_thumbnail(`upload.png`, `thumb.jpg`, 200));";
        "image_crop" [Image, Tokio] => "std_lib::image::crop", (from_path: s, to_path: s, x: i, y: i, width: i, height: i) -> (v!e),
            "Writes the rectangle of the picture starting at that corner, measured from the top left in pixels. A rectangle reaching past the edge is an error rather than a smaller crop.",
            "danger(image_crop(`photo.jpg`, `banner.jpg`, 0, 120, 1200, 400));";
        "image_grayscale" [Image, Tokio] => "std_lib::image::grayscale", (from_path: s, to_path: s) -> (v!e),
            "Writes the picture in shades of grey, weighted the way an eye weighs colours rather than averaged, so the greys come out at the brightness the colours looked.",
            "danger(image_grayscale(`photo.jpg`, `print.jpg`));";
        "image_blur" [Image, Tokio] => "std_lib::image::blur", (from_path: s, to_path: s, radius: f) -> (v!e),
            "Writes the picture blurred by that many pixels, for a background behind text or a preview that loads before the real thing. A larger radius is slower.",
            "danger(image_blur(`photo.jpg`, `backdrop.jpg`, 8.0));";
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

    // Turning takes the IMAGE_Turn enum, which needs a custom type import, so
    // it uses the full struct form rather than simple_fns.
    m.insert("image_rotate", StdlibFunction {
        rust_path: "std_lib::image::rotate".to_string(),
        crate_deps: vec![CrateDependency::Image, CrateDependency::Tokio, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("IMAGE_Turn", "nail::std_lib::image")],
        module: StdlibModule::Image,
        parameters: vec![
            nail_param!(from_path: s),
            nail_param!(to_path: s),
            StdlibParameter { name: "turn".to_string(), param_type: NailDataTypeDescriptor::Enum("IMAGE_Turn".to_string()), pass_by_reference: false },
        ],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Writes the picture turned a quarter, a half or three quarters round, for a photograph that came off a phone on its side. Only the quarter turns exist: any other angle leaves empty corners nothing can fill sensibly.",
        example: "danger(image_rotate(`scan.jpg`, `upright.jpg`, IMAGE_Turn::Clockwise));",
    });
}
