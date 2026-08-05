//! Pictures, resized and measured by path.
//!
//! A profile photo someone uploads is four megapixels and needs to be a
//! thumbnail before it goes in a page. That is the whole of what a program
//! normally does to an image, and it is a file-to-file operation: read that
//! path, write this one. Nothing binary crosses into Nail, which is why this
//! works without a bytes type.
//!
//! The format is decided by the extension of the path written to, so
//! `image_resize(`upload.png`, `thumb.jpg`, ...)` converts as well as resizes.
//! Reading handles whatever the `image` crate does - PNG, JPEG, GIF, WebP, BMP,
//! TIFF - and reads the actual bytes rather than trusting the extension.
//!
//! What is deliberately absent: anything that works on pixels one at a time.
//! That needs the picture in memory as values, and a Nail program has no use for
//! four million of them.

/// Longest side either dimension may be asked for. A request beyond this is
/// almost always a typo or an attack, and a resize to 100000 by 100000 would
/// allocate forty gigabytes before failing.
const LARGEST_SIDE: i64 = 20_000;

fn open(path: &str, function_name: &str) -> Result<image::DynamicImage, String> {
    return image::open(path).map_err(|failure| match failure {
        image::ImageError::IoError(io_failure) => format!("{}: could not read '{}': {}", function_name, path, io_failure),
        // The bytes decide the format, so a JPEG named `.png` still opens - what
        // fails here is a file that is not a picture at all.
        other => format!("{}: '{}' is not a picture this can read: {}", function_name, path, other),
    });
}

/// Writes a copy of the picture at the given size. The size is exact, so a
/// picture whose shape does not match will be stretched - `image_resize_within`
/// is the one that keeps the proportions.
pub async fn resize(from_path: String, to_path: String, width: i64, height: i64) -> Result<(), String> {
    if width < 1 || height < 1 {
        return Err(format!("image_resize: a picture {} by {} has no area, so there is nothing to write", width, height));
    }
    if width > LARGEST_SIDE || height > LARGEST_SIDE {
        return Err(format!("image_resize: {} by {} is larger than the largest side this allows, {}", width, height, LARGEST_SIDE));
    }

    let picture = open(&from_path, "image_resize")?;
    // Lanczos3 rather than nearest-neighbour: this is almost always making
    // something smaller, and the cheap filters make text in a screenshot
    // unreadable.
    let resized = picture.resize_exact(width as u32, height as u32, image::imageops::FilterType::Lanczos3);
    return resized.save(&to_path).map_err(|failure| format!("image_resize: could not write '{}': {}", to_path, failure));
}

/// Writes a copy that fits inside the given box without being stretched: the
/// proportions are kept and one side comes out smaller than asked for. What a
/// thumbnail wants, since a stretched face is worse than a small one.
///
/// A picture already inside the box is copied at its own size rather than being
/// enlarged - making a small picture bigger adds no detail, only bytes.
pub async fn resize_within(from_path: String, to_path: String, width: i64, height: i64) -> Result<(), String> {
    if width < 1 || height < 1 {
        return Err(format!("image_resize_within: a box {} by {} has no area, so there is nothing to write", width, height));
    }
    if width > LARGEST_SIDE || height > LARGEST_SIDE {
        return Err(format!("image_resize_within: {} by {} is larger than the largest side this allows, {}", width, height, LARGEST_SIDE));
    }

    let picture = open(&from_path, "image_resize_within")?;
    // The crate's own `resize` scales up as well as down. Enlarging adds no
    // detail and only bytes, so a picture already inside the box is written at
    // the size it came in at.
    let already_fits = picture.width() <= width as u32 && picture.height() <= height as u32;
    let resized = if already_fits { picture } else { picture.resize(width as u32, height as u32, image::imageops::FilterType::Lanczos3) };
    return resized.save(&to_path).map_err(|failure| format!("image_resize_within: could not write '{}': {}", to_path, failure));
}

/// A copy of the picture in whatever format the written path's extension names -
/// a PNG upload stored as a smaller JPEG, a photo turned into a WebP for a page.
pub async fn convert(from_path: String, to_path: String) -> Result<(), String> {
    let picture = open(&from_path, "image_convert")?;
    return picture.save(&to_path).map_err(|failure| format!("image_convert: could not write '{}': {}", to_path, failure));
}

pub async fn width(path: String) -> Result<i64, String> {
    let (width, _) = image::image_dimensions(&path).map_err(|failure| format!("image_width: could not read the size of '{}': {}", path, failure))?;
    return Ok(width as i64);
}

pub async fn height(path: String) -> Result<i64, String> {
    let (_, height) = image::image_dimensions(&path).map_err(|failure| format!("image_height: could not read the size of '{}': {}", path, failure))?;
    return Ok(height as i64);
}

/// What format the file actually is, read from its bytes rather than its name:
/// `png`, `jpeg`, `gif`, `webp` and so on. This is how a program checks that an
/// upload claiming to be an image is one, before storing it under a name a
/// browser will trust.
pub async fn format(path: String) -> Result<String, String> {
    let reader = image::ImageReader::open(&path).map_err(|failure| format!("image_format: could not read '{}': {}", path, failure))?;
    let guessed = reader.with_guessed_format().map_err(|failure| format!("image_format: could not read '{}': {}", path, failure))?;
    return match guessed.format() {
        Some(format) => Ok(format!("{:?}", format).to_lowercase()),
        None => Err(format!("image_format: '{}' is not a picture in any format this knows", path)),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture to work from, written where the tests can find it. Made rather
    /// than checked in, so nothing binary lives in the repository.
    fn a_picture(name: &str, width: u32, height: u32) -> String {
        let path = std::env::temp_dir().join(format!("nail_image_{}.png", name));
        let mut picture = image::RgbImage::new(width, height);
        for (x, y, pixel) in picture.enumerate_pixels_mut() {
            *pixel = image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]);
        }
        picture.save(&path).expect("a writable temporary picture");
        return path.to_string_lossy().to_string();
    }

    fn beside(name: &str) -> String {
        let path = std::env::temp_dir().join(name);
        let _ = std::fs::remove_file(&path);
        return path.to_string_lossy().to_string();
    }

    #[tokio::test]
    async fn a_picture_is_measured() {
        let source = a_picture("measured", 40, 20);
        assert_eq!(width(source.clone()).await.expect("a readable picture"), 40);
        assert_eq!(height(source.clone()).await.expect("a readable picture"), 20);
        let _ = std::fs::remove_file(&source);
    }

    #[tokio::test]
    async fn resizing_writes_exactly_the_size_asked_for() {
        let source = a_picture("resized", 40, 20);
        let thumbnail = beside("nail_image_resized_thumb.png");
        resize(source.clone(), thumbnail.clone(), 10, 10).await.expect("a writable thumbnail");
        assert_eq!(width(thumbnail.clone()).await.expect("a readable picture"), 10);
        assert_eq!(height(thumbnail.clone()).await.expect("a readable picture"), 10);
        let _ = std::fs::remove_file(&source);
        let _ = std::fs::remove_file(&thumbnail);
    }

    #[tokio::test]
    async fn fitting_inside_a_box_keeps_the_proportions() {
        let source = a_picture("within", 40, 20);
        let thumbnail = beside("nail_image_within_thumb.png");
        resize_within(source.clone(), thumbnail.clone(), 10, 10).await.expect("a writable thumbnail");
        // Twice as wide as it is tall, so the width is the side that fills the box.
        assert_eq!(width(thumbnail.clone()).await.expect("a readable picture"), 10);
        assert_eq!(height(thumbnail.clone()).await.expect("a readable picture"), 5);
        let _ = std::fs::remove_file(&source);
        let _ = std::fs::remove_file(&thumbnail);
    }

    #[tokio::test]
    async fn a_picture_already_inside_the_box_is_not_enlarged() {
        let source = a_picture("small", 8, 4);
        let copy = beside("nail_image_small_copy.png");
        resize_within(source.clone(), copy.clone(), 100, 100).await.expect("a writable copy");
        assert_eq!(width(copy.clone()).await.expect("a readable picture"), 8);
        let _ = std::fs::remove_file(&source);
        let _ = std::fs::remove_file(&copy);
    }

    #[tokio::test]
    async fn the_written_extension_decides_the_format() {
        let source = a_picture("converted", 12, 12);
        let as_jpeg = beside("nail_image_converted.jpg");
        convert(source.clone(), as_jpeg.clone()).await.expect("a writable copy");
        assert_eq!(format(as_jpeg.clone()).await.expect("a readable picture"), "jpeg");
        assert_eq!(format(source.clone()).await.expect("a readable picture"), "png");
        let _ = std::fs::remove_file(&source);
        let _ = std::fs::remove_file(&as_jpeg);
    }

    /// The bytes decide what a file is, not its name - which is the check worth
    /// doing on an upload.
    #[tokio::test]
    async fn the_format_is_read_from_the_bytes_not_the_name() {
        let source = a_picture("mislabelled", 10, 10);
        let lying = beside("nail_image_mislabelled.jpg");
        std::fs::copy(&source, &lying).expect("a writable copy");
        assert_eq!(format(lying.clone()).await.expect("a readable picture"), "png");
        let _ = std::fs::remove_file(&source);
        let _ = std::fs::remove_file(&lying);
    }

    #[tokio::test]
    async fn something_that_is_not_a_picture_says_so() {
        let text = beside("nail_image_not_a_picture.png");
        std::fs::write(&text, "this is just text").expect("a writable file");
        let failure = resize(text.clone(), beside("nail_image_never.png"), 10, 10).await.unwrap_err();
        assert!(failure.contains("is not a picture this can read"), "got: {}", failure);
        assert!(width(text.clone()).await.is_err());
        let _ = std::fs::remove_file(&text);
    }

    #[tokio::test]
    async fn a_file_that_is_not_there_says_so() {
        let failure = resize("/tmp/nail_no_such_picture.png".to_string(), beside("nail_image_never.png"), 10, 10).await.unwrap_err();
        assert!(failure.contains("could not read"), "got: {}", failure);
    }

    #[tokio::test]
    async fn a_size_that_makes_no_sense_is_refused() {
        let source = a_picture("refused", 10, 10);
        assert!(resize(source.clone(), beside("nail_image_never.png"), 0, 10).await.is_err());
        assert!(resize(source.clone(), beside("nail_image_never.png"), 10, -1).await.is_err());
        let too_big = resize(source.clone(), beside("nail_image_never.png"), 50_000, 10).await.unwrap_err();
        assert!(too_big.contains("largest side"), "got: {}", too_big);
        let _ = std::fs::remove_file(&source);
    }
}
