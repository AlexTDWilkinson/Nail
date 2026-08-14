//! A window, a picture in it, and a loop - the whole of a 2D game.
//!
//! This is the windowed sibling of `tui_run`, and it works the same way: the
//! program is two functions and a state struct. `update(state, input)` returns
//! the next state, `view(state)` returns what the frame looks like, and
//! `game_run` owns everything in between - the window, the keyboard and
//! mouse, drawing, and pacing. There is no engine object to hold and no
//! callback to register. A frame is a value: an array of shapes, painted in
//! order, later shapes over earlier ones, exactly like the draw module's SVG.
//!
//! Everything under this is pure Rust (winit for the window, tiny-skia for
//! the pixels, fontdue for text), so building a game is `cargo build` and
//! nothing else - no C toolchain, no system libraries to install.
//!
//! Coordinates are pixels, starting at the top left with y growing downward.
//! Colours are strings: `#rrggbb`, `#rrggbbaa`, or a basic name like `red`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};

/// How the window starts out. `target_fps` is honoured as written: the
/// loop paces to whatever the config asks for, however high, and 0 runs
/// unpaced - every frame the machine can make, a spinning core by choice,
/// which is what a high refresh monitor wants. In a browser the compositor
/// paces frames through requestAnimationFrame whatever this says, so the
/// display's refresh rate is the ceiling there.
/// `pixel_size` is how many screen pixels one drawn pixel covers: 1 is
/// full resolution, 2 draws at half size and scales up chunky, which
/// quarters the pixels the CPU rasterizer has to fill. Coordinates in the
/// game stay in window pixels whatever the value.
///
/// `physics_hz` is how many times a second `update` runs. At 0 it runs once
/// a frame and is handed however long that frame really took, which is
/// simplest and fine for anything that is not simulating. Set it and update
/// instead runs in fixed slices, as many as the elapsed time has banked, so
/// the same jump clears the same gap on a slow machine and a fast one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAME_Config {
    pub title: String,
    pub width: i64,
    pub height: i64,
    pub target_fps: i64,
    pub pixel_size: i64,
    pub physics_hz: i64,
}

/// One thing to paint. Programs never fill this in by hand - the constructor
/// functions (`game_rect`, `game_circle`, ...) each set the fields their
/// shape actually uses and zero the rest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAME_Shape {
    pub kind: String,
    pub x_coordinate: f64,
    pub y_coordinate: f64,
    pub width: f64,
    pub height: f64,
    pub end_x: f64,
    pub end_y: f64,
    pub third_x: f64,
    pub third_y: f64,
    pub radius: f64,
    pub thickness: f64,
    pub color: String,
    pub text: String,
    pub size: f64,
    pub sprite: i64,
}

/// What one frame looks like: a background colour, shapes painted over it in
/// array order, and whether the game is finished.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAME_Frame {
    pub shapes: Vec<GAME_Shape>,
    pub background: String,
    pub quit: bool,
}

/// Everything the player did since the last frame. `keys_down` is every key
/// currently held, `keys_pressed` only the ones that went down this frame -
/// held movement reads the first, a jump reads the second. `delta_ms` is how
/// long the last frame really took, so movement multiplied by it stays the
/// same speed at any frame rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAME_Input {
    pub keys_down: Vec<String>,
    pub keys_pressed: Vec<String>,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub mouse_down: bool,
    pub mouse_right: bool,
    pub scroll: f64,
    pub delta_ms: f64,
    /// Every finger touching right now, as x then y for each one. The first
    /// finger also arrives as the mouse, so a game that only reads the mouse
    /// still works by touch, but a game that wants two controls at once, a
    /// direction pad and a jump button, has to look here. Empty on a desktop.
    pub touches: Vec<f64>,
}

pub(crate) fn blank(kind: &str) -> GAME_Shape {
    return GAME_Shape {
        kind: kind.to_string(),
        x_coordinate: 0.0,
        y_coordinate: 0.0,
        width: 0.0,
        height: 0.0,
        end_x: 0.0,
        end_y: 0.0,
        third_x: 0.0,
        third_y: 0.0,
        radius: 0.0,
        thickness: 0.0,
        color: String::new(),
        text: String::new(),
        size: 0.0,
        sprite: 0,
    };
}

/// A filled rectangle with its top left corner at x, y.
pub fn rect(x: f64, y: f64, width: f64, height: f64, color: String) -> GAME_Shape {
    let mut shape = blank("rect");
    shape.x_coordinate = x;
    shape.y_coordinate = y;
    shape.width = width;
    shape.height = height;
    shape.color = color;
    return shape;
}

/// Just the border of a rectangle, `thickness` pixels wide.
pub fn rect_outline(x: f64, y: f64, width: f64, height: f64, thickness: f64, color: String) -> GAME_Shape {
    let mut shape = blank("rect_outline");
    shape.x_coordinate = x;
    shape.y_coordinate = y;
    shape.width = width;
    shape.height = height;
    shape.thickness = thickness;
    shape.color = color;
    return shape;
}

/// A filled circle centred on x, y.
pub fn circle(x: f64, y: f64, radius: f64, color: String) -> GAME_Shape {
    let mut shape = blank("circle");
    shape.x_coordinate = x;
    shape.y_coordinate = y;
    shape.radius = radius;
    shape.color = color;
    return shape;
}

/// A straight line from one point to another, `thickness` pixels wide.
pub fn line(x: f64, y: f64, x2: f64, y2: f64, thickness: f64, color: String) -> GAME_Shape {
    let mut shape = blank("line");
    shape.x_coordinate = x;
    shape.y_coordinate = y;
    shape.end_x = x2;
    shape.end_y = y2;
    shape.thickness = thickness;
    shape.color = color;
    return shape;
}

/// A filled triangle. The 3D module emits these, and they are just as
/// usable straight from a program.
pub fn triangle(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, color: String) -> GAME_Shape {
    let mut shape = blank("triangle");
    shape.x_coordinate = x1;
    shape.y_coordinate = y1;
    shape.end_x = x2;
    shape.end_y = y2;
    shape.third_x = x3;
    shape.third_y = y3;
    shape.color = color;
    return shape;
}

/// Text whose top left corner is at x, y, `size` pixels tall.
pub fn text(content: String, x: f64, y: f64, size: f64, color: String) -> GAME_Shape {
    let mut shape = blank("text");
    shape.text = content;
    shape.x_coordinate = x;
    shape.y_coordinate = y;
    shape.size = size;
    shape.color = color;
    return shape;
}

/// A loaded sprite drawn at its own size with its top left corner at x, y.
pub fn sprite(handle: i64, x: f64, y: f64) -> GAME_Shape {
    let mut shape = blank("sprite");
    shape.sprite = handle;
    shape.x_coordinate = x;
    shape.y_coordinate = y;
    return shape;
}

/// A loaded sprite stretched to `width` by `height` at x, y.
pub fn sprite_scaled(handle: i64, x: f64, y: f64, width: f64, height: f64) -> GAME_Shape {
    let mut shape = blank("sprite_scaled");
    shape.sprite = handle;
    shape.x_coordinate = x;
    shape.y_coordinate = y;
    shape.width = width;
    shape.height = height;
    return shape;
}

/// Loaded sprites live here so a shape can name one with a plain number.
fn sprites() -> &'static Mutex<HashMap<i64, tiny_skia::Pixmap>> {
    static SPRITES: OnceLock<Mutex<HashMap<i64, tiny_skia::Pixmap>>> = OnceLock::new();
    return SPRITES.get_or_init(|| Mutex::new(HashMap::new()));
}

static NEXT_SPRITE: AtomicI64 = AtomicI64::new(1);

/// Reads a PNG from disk and returns the number that names it from now on.
/// Load sprites once before `game_run`, not inside update or view.
#[cfg(not(target_arch = "wasm32"))]
pub fn sprite_load(path: String) -> Result<i64, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("game_sprite_load: could not read {}: {}", path, e))?;
    let pixmap = tiny_skia::Pixmap::decode_png(&bytes).map_err(|e| format!("game_sprite_load: {} is not a PNG this understands: {}", path, e))?;
    let handle = NEXT_SPRITE.fetch_add(1, Ordering::Relaxed);
    sprites().lock().map_err(|_| "game_sprite_load: the sprite store is poisoned".to_string())?.insert(handle, pixmap);
    return Ok(handle);
}

/// A browser has no disk to read a PNG from, so in the wasm build this can
/// only explain itself. Sprites on the web will come from fetch later.
#[cfg(target_arch = "wasm32")]
pub fn sprite_load(path: String) -> Result<i64, String> {
    return Err(format!("game_sprite_load: the browser build cannot read {} from disk - draw with shapes for now", path));
}

/// The font every `game_text` shape is drawn in, parsed once from the bytes
/// baked into the compiler at build time.
fn font() -> Result<&'static fontdue::Font, String> {
    static FONT: OnceLock<Option<fontdue::Font>> = OnceLock::new();
    let parsed = FONT.get_or_init(|| {
        let bytes: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSansMono.ttf");
        return fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok();
    });
    return parsed.as_ref().ok_or_else(|| "game_run: the built-in font failed to parse, which means this build of nail is broken".to_string());
}

/// Turns a colour string into an actual colour, accepting `#rgb`, `#rrggbb`,
/// `#rrggbbaa` and a handful of plain English names.
pub(crate) fn parse_color(name: &str) -> Result<tiny_skia::Color, String> {
    let named = match name {
        "black" => Some((0x00, 0x00, 0x00)),
        "white" => Some((0xff, 0xff, 0xff)),
        "red" => Some((0xe5, 0x39, 0x35)),
        "green" => Some((0x43, 0xa0, 0x47)),
        "blue" => Some((0x1e, 0x88, 0xe5)),
        "yellow" => Some((0xfd, 0xd8, 0x35)),
        "orange" => Some((0xfb, 0x8c, 0x00)),
        "purple" => Some((0x8e, 0x24, 0xaa)),
        "cyan" => Some((0x00, 0xac, 0xc1)),
        "magenta" => Some((0xd8, 0x1b, 0x60)),
        "gray" => Some((0x75, 0x75, 0x75)),
        _ => None,
    };
    if let Some((r, g, b)) = named {
        return Ok(tiny_skia::Color::from_rgba8(r, g, b, 0xff));
    }

    let hex = match name.strip_prefix('#') {
        Some(rest) => rest,
        None => return Err(format!("game: `{}` is not a colour this understands - use `#rrggbb`, `#rrggbbaa` or a basic name like `red`", name)),
    };
    let digit = |index: usize| -> Result<u8, String> {
        let slice = hex.get(index..index + 2).ok_or_else(|| format!("game: `{}` is not a colour this understands - a hex colour needs 3, 6 or 8 digits", name))?;
        return u8::from_str_radix(slice, 16).map_err(|_| format!("game: `{}` has a digit that is not hex", name));
    };
    return match hex.len() {
        3 => {
            let one = |index: usize| -> Result<u8, String> {
                let slice = hex.get(index..index + 1).ok_or_else(|| format!("game: `{}` is not a colour this understands", name))?;
                let value = u8::from_str_radix(slice, 16).map_err(|_| format!("game: `{}` has a digit that is not hex", name))?;
                return Ok(value * 17);
            };
            Ok(tiny_skia::Color::from_rgba8(one(0)?, one(1)?, one(2)?, 0xff))
        }
        6 => Ok(tiny_skia::Color::from_rgba8(digit(0)?, digit(2)?, digit(4)?, 0xff)),
        8 => Ok(tiny_skia::Color::from_rgba8(digit(0)?, digit(2)?, digit(4)?, digit(6)?)),
        _ => Err(format!("game: `{}` is not a colour this understands - a hex colour needs 3, 6 or 8 digits", name)),
    };
}

/// The clamped chunky-pixel factor from a config: at least 1, at most 8,
/// and 0 (an unset-feeling value) means full resolution.
fn pixel_size_of(config: &GAME_Config) -> u32 {
    return config.pixel_size.clamp(1, 8) as u32;
}

/// One sprite's pixels, premultiplied RGBA, for the graphics card path to
/// upload once. None when no sprite has that number.
pub(crate) fn sprite_pixels(handle: i64) -> Option<(Vec<u8>, u32, u32)> {
    let store = sprites().lock().ok()?;
    let pixmap = store.get(&handle)?;
    return Some((pixmap.data().to_vec(), pixmap.width(), pixmap.height()));
}

/// The built-in font, for the graphics card path's glyph atlas.
pub(crate) fn game_font() -> Result<&'static fontdue::Font, String> {
    return font();
}

/// A shape's loose bounding box, for skipping what is not on screen. None
/// means the bounds are unknown (text and unscaled sprites) and the shape
/// always draws.
pub(crate) fn shape_bounds(shape: &GAME_Shape) -> Option<(f64, f64, f64, f64)> {
    match shape.kind.as_str() {
        "rect" | "sprite_scaled" => Some((shape.x_coordinate, shape.y_coordinate, shape.x_coordinate + shape.width, shape.y_coordinate + shape.height)),
        "rect_outline" => {
            let pad = shape.thickness / 2.0;
            Some((shape.x_coordinate - pad, shape.y_coordinate - pad, shape.x_coordinate + shape.width + pad, shape.y_coordinate + shape.height + pad))
        }
        "circle" => Some((shape.x_coordinate - shape.radius, shape.y_coordinate - shape.radius, shape.x_coordinate + shape.radius, shape.y_coordinate + shape.radius)),
        "line" => Some((
            shape.x_coordinate.min(shape.end_x) - shape.thickness,
            shape.y_coordinate.min(shape.end_y) - shape.thickness,
            shape.x_coordinate.max(shape.end_x) + shape.thickness,
            shape.y_coordinate.max(shape.end_y) + shape.thickness,
        )),
        "triangle" => Some((
            shape.x_coordinate.min(shape.end_x).min(shape.third_x),
            shape.y_coordinate.min(shape.end_y).min(shape.third_y),
            shape.x_coordinate.max(shape.end_x).max(shape.third_x),
            shape.y_coordinate.max(shape.end_y).max(shape.third_y),
        )),
        _ => None,
    }
}

/// Fills one triangle shape into the pixmap. The 2D triangle branch and
/// every triangle of an expanded 3D scene come through here.
fn fill_triangle(pixmap: &mut tiny_skia::Pixmap, shape: &GAME_Shape, anti_alias: bool, transform: tiny_skia::Transform) -> Result<(), String> {
    let mut builder = tiny_skia::PathBuilder::new();
    builder.move_to(shape.x_coordinate as f32, shape.y_coordinate as f32);
    builder.line_to(shape.end_x as f32, shape.end_y as f32);
    builder.line_to(shape.third_x as f32, shape.third_y as f32);
    builder.close();
    let Some(path) = builder.finish() else { return Ok(()) };
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(parse_color(&shape.color)?);
    paint.anti_alias = anti_alias;
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
    return Ok(());
}

/// Paints one frame's shapes into the pixmap. Shapes speak window pixels
/// whatever `pixel_size` says, the scale transform is what maps them onto
/// the possibly smaller pixmap.
fn rasterize(pixmap: &mut tiny_skia::Pixmap, overlay: Option<&mut tiny_skia::Pixmap>, frame: &GAME_Frame, pixel_size: u32) -> Result<(), String> {
    pixmap.fill(parse_color(&frame.background)?);
    return rasterize_shapes(pixmap, overlay, &frame.shapes, pixel_size);
}

/// Paints a run of shapes into the pixmap, over whatever is already there.
/// The whole-frame path above fills the background first, the graphics
/// card path hands in transparent layers one run at a time.
pub(crate) fn rasterize_shapes(pixmap: &mut tiny_skia::Pixmap, mut overlay: Option<&mut tiny_skia::Pixmap>, shapes: &[GAME_Shape], pixel_size: u32) -> Result<(), String> {
    let scale = 1.0 / pixel_size as f32;
    let logical_width = (pixmap.width() * pixel_size) as f64;
    let logical_height = (pixmap.height() * pixel_size) as f64;
    // Chunky rendering wants hard edges. Antialiased edges in the small
    // buffer become two-pixel smears once upscaled, which reads as blur
    // instead of pixel art, so above pixel_size 1 the edges go crisp.
    let anti_alias = pixel_size == 1;

    for shape in shapes {
        // A scrolling game hands over its whole world every frame, parallax
        // layers included, and most of it is off screen. Skipping here is
        // cheaper than letting the rasterizer clip path by path.
        if let Some((min_x, min_y, max_x, max_y)) = shape_bounds(shape) {
            if max_x < 0.0 || min_x > logical_width || max_y < 0.0 || min_y > logical_height {
                continue;
            }
        }
        let identity = tiny_skia::Transform::from_scale(scale, scale);
        match shape.kind.as_str() {
            "rect" => {
                let Some(rect) = tiny_skia::Rect::from_xywh(shape.x_coordinate as f32, shape.y_coordinate as f32, shape.width as f32, shape.height as f32) else { continue };
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(parse_color(&shape.color)?);
                paint.anti_alias = anti_alias;
                pixmap.fill_rect(rect, &paint, identity, None);
            }
            "rect_outline" => {
                let Some(rect) = tiny_skia::Rect::from_xywh(shape.x_coordinate as f32, shape.y_coordinate as f32, shape.width as f32, shape.height as f32) else { continue };
                let path = tiny_skia::PathBuilder::from_rect(rect);
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(parse_color(&shape.color)?);
                paint.anti_alias = anti_alias;
                let stroke = tiny_skia::Stroke { width: shape.thickness as f32, ..tiny_skia::Stroke::default() };
                pixmap.stroke_path(&path, &paint, &stroke, identity, None);
            }
            "circle" => {
                let Some(path) = tiny_skia::PathBuilder::from_circle(shape.x_coordinate as f32, shape.y_coordinate as f32, shape.radius as f32) else { continue };
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(parse_color(&shape.color)?);
                paint.anti_alias = anti_alias;
                pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, identity, None);
            }
            "line" => {
                let mut builder = tiny_skia::PathBuilder::new();
                builder.move_to(shape.x_coordinate as f32, shape.y_coordinate as f32);
                builder.line_to(shape.end_x as f32, shape.end_y as f32);
                let Some(path) = builder.finish() else { continue };
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(parse_color(&shape.color)?);
                paint.anti_alias = anti_alias;
                let stroke = tiny_skia::Stroke { width: shape.thickness as f32, ..tiny_skia::Stroke::default() };
                pixmap.stroke_path(&path, &paint, &stroke, identity, None);
            }
            "triangle" => {
                fill_triangle(pixmap, shape, anti_alias, identity)?;
            }
            "scene3d" => {
                // The CPU fallback for a whole 3D scene: expand it to
                // painter-ordered triangles on the spot. Each scene is
                // taken out of its store as it draws, which is also what
                // keeps the store from growing across frames.
                let Some(data) = super::game3d::take_scene(shape.sprite) else {
                    return Err("game_run: a scene3d shape refers to a scene that was already drawn - game3d_scene makes a fresh one each frame".to_string());
                };
                for triangle in super::game3d::expand_scene(&data)? {
                    fill_triangle(pixmap, &triangle, anti_alias, identity)?;
                }
            }
            "text" => {
                match overlay.as_deref_mut() {
                    // Text goes on the full resolution overlay when there is
                    // one, smooth and sized in window pixels.
                    Some(full) => draw_text(full, shape, 1.0, false)?,
                    None => draw_text(pixmap, shape, scale, !anti_alias)?,
                }
            }
            "sprite" | "sprite_scaled" => {
                let store = sprites().lock().map_err(|_| "game_run: the sprite store is poisoned".to_string())?;
                let Some(loaded) = store.get(&shape.sprite) else {
                    return Err(format!("game_run: shape refers to sprite {} but no sprite with that number was loaded", shape.sprite));
                };
                let transform = if shape.kind == "sprite_scaled" && loaded.width() > 0 && loaded.height() > 0 {
                    let scale_x = shape.width as f32 / loaded.width() as f32;
                    let scale_y = shape.height as f32 / loaded.height() as f32;
                    tiny_skia::Transform::from_scale(scale_x, scale_y).post_translate(shape.x_coordinate as f32, shape.y_coordinate as f32).post_scale(scale, scale)
                } else {
                    tiny_skia::Transform::from_translate(shape.x_coordinate as f32, shape.y_coordinate as f32).post_scale(scale, scale)
                };
                pixmap.draw_pixmap(0, 0, loaded.as_ref(), &tiny_skia::PixmapPaint::default(), transform, None);
            }
            other => {
                return Err(format!("game_run: `{}` is not a shape kind this understands", other));
            }
        }
    }
    return Ok(());
}

/// Draws one text shape glyph by glyph, blending each coverage bitmap from
/// fontdue straight into the pixmap.
fn draw_text(pixmap: &mut tiny_skia::Pixmap, shape: &GAME_Shape, scale: f32, crisp: bool) -> Result<(), String> {
    let font = font()?;
    let color = parse_color(&shape.color)?;
    let red = (color.red() * 255.0) as u16;
    let green = (color.green() * 255.0) as u16;
    let blue = (color.blue() * 255.0) as u16;
    // Glyphs rasterize at the already scaled size, so chunky-pixel text is
    // shaped for the small buffer instead of shrunk after the fact.
    let size = shape.size as f32 * scale;
    // The shape's y is the top of the text, glyphs hang from the baseline
    // below it. The size itself is a workable ascent for one line.
    let baseline = shape.y_coordinate as f32 * scale + size;
    let mut cursor = shape.x_coordinate as f32 * scale;
    let pixmap_width = pixmap.width() as i32;
    let pixmap_height = pixmap.height() as i32;

    for character in shape.text.chars() {
        let (metrics, coverage) = font.rasterize(character, size);
        let glyph_left = cursor as i32 + metrics.xmin;
        let glyph_top = baseline as i32 - metrics.height as i32 - metrics.ymin;
        let data = pixmap.data_mut();
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                // Crisp mode snaps glyph coverage to on or off, a bitmap
                // font look that upscales into sharp blocks instead of fuzz.
                let alpha = if crisp {
                    if coverage[row * metrics.width + column] < 128 { 0u16 } else { 255u16 }
                } else {
                    coverage[row * metrics.width + column] as u16
                };
                if alpha == 0 {
                    continue;
                }
                let x = glyph_left + column as i32;
                let y = glyph_top + row as i32;
                if x < 0 || y < 0 || x >= pixmap_width || y >= pixmap_height {
                    continue;
                }
                let index = (y as usize * pixmap_width as usize + x as usize) * 4;
                // Premultiplied source-over blend, the same arithmetic
                // tiny-skia uses for its own shapes.
                let inverse = 255 - alpha;
                data[index] = ((red * alpha + data[index] as u16 * inverse) / 255) as u8;
                data[index + 1] = ((green * alpha + data[index + 1] as u16 * inverse) / 255) as u8;
                data[index + 2] = ((blue * alpha + data[index + 2] as u16 * inverse) / 255) as u8;
                data[index + 3] = ((255 * alpha + data[index + 3] as u16 * inverse) / 255) as u8;
            }
        }
        cursor += metrics.advance_width;
    }
    return Ok(());
}

pub type ViewFuture = Pin<Box<dyn Future<Output = GAME_Frame> + Send>>;
pub type UpdateFuture<S> = Pin<Box<dyn Future<Output = S> + Send>>;

/// What one poll of the backend reported: whether the player asked to close,
/// whether the drawing surface exists yet, and everything the player did
/// since the last poll. `keys_down` arrives already sorted.
struct Poll {
    close_requested: bool,
    ready: bool,
    keys_down: Vec<String>,
    keys_pressed: Vec<String>,
    mouse_x: f64,
    mouse_y: f64,
    mouse_down: bool,
    mouse_right: bool,
    scroll: f64,
    touches: Vec<f64>,
}

#[cfg(not(target_arch = "wasm32"))]
use native_backend as backend;
#[cfg(target_arch = "wasm32")]
use web_backend as backend;

/// Opens the window and runs the game until its view reports `quit` or the
/// player closes the window, and returns the state it finished with.
///
/// The loop is: hand the player's input to `update`, draw what `view` says,
/// wait out the rest of the frame, repeat. Waiting is async, so the runtime
/// this shares a thread with keeps serving anything else the program spawned.
///
/// This one loop runs on every target. Everything platform-shaped sits
/// behind the backend chosen at compile time: where the picture goes, where
/// input comes from, what clock time is read from, and how the gap between
/// frames is waited out.
pub async fn run<S, V, U, B>(config: GAME_Config, initial: S, view: V, update: U, blend: B) -> Result<S, String>
where
    S: Clone + Send + 'static,
    V: Fn(S) -> ViewFuture + Send + Sync + 'static,
    U: Fn(S, GAME_Input) -> UpdateFuture<S> + Send + Sync + 'static,
    B: Fn(S, S, f64) -> UpdateFuture<S> + Send + Sync + 'static,
{
    let width = u32::try_from(config.width).ok().filter(|size| *size > 0).ok_or_else(|| format!("game_run: {} is not a width a {} can have", config.width, backend::SURFACE_NOUN))?;
    let height = u32::try_from(config.height).ok().filter(|size| *size > 0).ok_or_else(|| format!("game_run: {} is not a height a {} can have", config.height, backend::SURFACE_NOUN))?;

    // The config's target is honoured as written: a target_fps of 0 runs
    // unpaced, exactly as game_run's documentation promises, and a 400Hz
    // monitor is allowed to be one. The browser backend paces by
    // requestAnimationFrame and ignores this.
    let paced_fps = config.target_fps.max(0);

    // Every game carries a frame counter in its top right corner, on
    // whichever renderer is drawing. NAIL_GAME_NO_FPS=1 hides it.
    #[cfg(not(target_arch = "wasm32"))]
    let show_fps = !std::env::var("NAIL_GAME_NO_FPS").map(|value| value == "1").unwrap_or(false);
    #[cfg(target_arch = "wasm32")]
    let show_fps = true;
    let mut smoothed_fps = 0.0_f64;

    let mut backend = backend::Backend::create(&config, width, height).await?;
    let pixel_size = pixel_size_of(&config);
    let mut pixmap = tiny_skia::Pixmap::new((width / pixel_size).max(1), (height / pixel_size).max(1)).ok_or_else(|| "game_run: could not make the frame".to_string())?;
    // Chunky worlds keep their text readable: glyphs render at full window
    // resolution on this transparent overlay and the backend composites it
    // over the upscaled world, the way pixel art games draw their UI.
    let mut overlay = if pixel_size > 1 {
        Some(tiny_skia::Pixmap::new(width, height).ok_or_else(|| "game_run: could not make the text overlay".to_string())?)
    } else {
        None
    };

    let mut state = initial;
    let started_ms = backend.now_ms();
    let mut last_ms = started_ms;
    let mut frames: u64 = 0;
    let mut work_ms = 0.0_f64;

    // A fixed step hands update the same slice of time every time it runs, so
    // the arithmetic is identical whatever the frame rate does. Elapsed time
    // is banked and spent a whole step at a time, which is what makes a jump
    // clear the same gap on every machine.
    let step_ms = if config.physics_hz > 0 { 1000.0 / config.physics_hz as f64 } else { 0.0 };
    let mut banked_ms = 0.0_f64;
    // Presses and wheel turns are moments rather than states, so they wait
    // here until a step actually runs. A tap between two steps is then never
    // lost, and never counted twice when several steps run at once.
    let mut waiting_pressed: Vec<String> = Vec::new();
    let mut waiting_scroll = 0.0_f64;
    // The step before the current one. A frame usually lands part way between
    // two steps, and drawing the newer one as it stands judders when the
    // screen refreshes faster than the physics runs, so the game is offered
    // both and says what part way looks like for its own data.
    let mut previous_state = state.clone();
    // Enough steps to catch up after a hitch, few enough that a long stall
    // does not send the game racing to make up minutes of missed time.
    const CATCH_UP_STEPS: u32 = 5;

    loop {
        let polled = backend.poll()?;
        if polled.close_requested {
            backend.report_close(frames, work_ms, backend.now_ms() - started_ms, config.target_fps);
            return Ok(state);
        }
        if !polled.ready {
            // The surface arrives through the platform's own first callback,
            // which can take a few polls. Idle briefly and ask again.
            backend.idle_wait().await;
            continue;
        }

        let frame_start_ms = backend.now_ms();
        let elapsed_ms = frame_start_ms - last_ms;
        last_ms = frame_start_ms;

        waiting_pressed.extend(polled.keys_pressed);
        waiting_scroll += polled.scroll;

        if step_ms > 0.0 {
            banked_ms += elapsed_ms;
            let mut steps = 0;
            while banked_ms >= step_ms && steps < CATCH_UP_STEPS {
                previous_state = state.clone();
                let input = GAME_Input {
                    keys_down: polled.keys_down.clone(),
                    keys_pressed: std::mem::take(&mut waiting_pressed),
                    mouse_x: polled.mouse_x,
                    mouse_y: polled.mouse_y,
                    mouse_down: polled.mouse_down,
                    mouse_right: polled.mouse_right,
                    scroll: std::mem::take(&mut waiting_scroll),
                    delta_ms: step_ms,
                    touches: polled.touches.clone(),
                };
                state = update(state, input).await;
                banked_ms -= step_ms;
                steps += 1;
            }
            if steps == CATCH_UP_STEPS {
                // The machine cannot keep up with the step rate this game
                // asked for. Drop the backlog rather than fall further behind
                // on every frame from here on.
                banked_ms = 0.0;
            }
        } else {
            let input = GAME_Input {
                keys_down: polled.keys_down,
                keys_pressed: std::mem::take(&mut waiting_pressed),
                mouse_x: polled.mouse_x,
                mouse_y: polled.mouse_y,
                mouse_down: polled.mouse_down,
                mouse_right: polled.mouse_right,
                scroll: std::mem::take(&mut waiting_scroll),
                delta_ms: elapsed_ms,
                touches: polled.touches,
            };
            state = update(state, input).await;
        }
        // What to draw: the current state as it stands when physics runs once
        // a frame, or the part way point between the last two steps when it
        // runs at its own rate.
        let drawn = if step_ms > 0.0 { blend(previous_state.clone(), state.clone(), (banked_ms / step_ms).clamp(0.0, 1.0)).await } else { state.clone() };
        let mut frame = view(drawn).await;
        if show_fps {
            // Smoothed over recent frames, so the number reads instead of
            // flickering. The shapes ride the frame itself, which is what
            // puts them on whichever renderer is drawing.
            let this_frame = 1000.0 / elapsed_ms.max(0.1);
            smoothed_fps = if smoothed_fps <= 0.0 { this_frame } else { smoothed_fps * 0.92 + this_frame * 0.08 };
            let corner = width as f64 - 78.0;
            frame.shapes.push(rect(corner - 6.0, 6.0, 78.0, 24.0, "#00000090".to_string()));
            frame.shapes.push(text(format!("{:>3.0} fps", smoothed_fps.min(999.0)), corner, 9.0, 16.0, "#7fdc7f".to_string()));
        }
        if let Some(full) = overlay.as_mut() {
            full.fill(tiny_skia::Color::TRANSPARENT);
        }
        backend.render(&frame, &mut pixmap, overlay.as_mut(), pixel_size).await?;
        // A scene the frame built but never put in a shape would otherwise
        // sit in the store forever.
        super::game3d::sweep_scenes();
        let used_ms = backend.now_ms() - frame_start_ms;
        frames += 1;
        work_ms += used_ms;
        if frame.quit {
            backend.report_close(frames, work_ms, backend.now_ms() - started_ms, paced_fps);
            return Ok(state);
        }

        backend.pace(used_ms, paced_fps).await?;
    }
}

/// The desktop backend: a winit window, softbuffer to put pixels in it,
/// tokio sleeps to pace the loop, and a monotonic Instant for the clock.
#[cfg(not(target_arch = "wasm32"))]
mod native_backend {
    use super::*;
    use std::collections::HashSet;
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use winit::application::ApplicationHandler;
    use winit::dpi::PhysicalSize;
    use winit::event::{ElementState, MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::keyboard::{Key, NamedKey};
    use winit::platform::pump_events::EventLoopExtPumpEvents;
    use winit::window::{Window, WindowId};

    /// The word the shared loop's size errors call the drawing surface.
    pub const SURFACE_NOUN: &str = "window";

    /// One name per key, shared by `keys_down` and `keys_pressed`. Letters and
    /// digits are themselves in lowercase, everything else is a word: `Up`,
    /// `Down`, `Left`, `Right`, `Space`, `Enter`, `Esc`, `Shift`, `Ctrl`, `Alt`,
    /// `Tab`, `Backspace`. Keys outside that set have no name and are not heard.
    fn key_name(key: &Key) -> String {
        return match key {
            Key::Character(character) => character.to_string().to_lowercase(),
            Key::Named(named) => match named {
                NamedKey::ArrowUp => "Up".to_string(),
                NamedKey::ArrowDown => "Down".to_string(),
                NamedKey::ArrowLeft => "Left".to_string(),
                NamedKey::ArrowRight => "Right".to_string(),
                NamedKey::Space => "Space".to_string(),
                NamedKey::Enter => "Enter".to_string(),
                NamedKey::Escape => "Esc".to_string(),
                NamedKey::Shift => "Shift".to_string(),
                NamedKey::Control => "Ctrl".to_string(),
                NamedKey::Alt => "Alt".to_string(),
                NamedKey::Tab => "Tab".to_string(),
                NamedKey::Backspace => "Backspace".to_string(),
                _ => String::new(),
            },
            _ => String::new(),
        };
    }

    /// The window and everything the player has done to it. winit drives this
    /// between frames through `pump_app_events`, the game loop reads and resets
    /// it after.
    struct App {
        title: String,
        width: u32,
        height: u32,
        window: Option<Arc<Window>>,
        surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
        keys_down: HashSet<String>,
        keys_pressed: Vec<String>,
        mouse_x: f64,
        mouse_y: f64,
        mouse_down: bool,
        mouse_right: bool,
        scroll: f64,
        close_requested: bool,
        startup_error: Option<String>,
    }

    impl App {
        fn new(title: String, width: u32, height: u32) -> App {
            return App {
                title,
                width,
                height,
                window: None,
                surface: None,
                keys_down: HashSet::new(),
                keys_pressed: Vec::new(),
                mouse_x: 0.0,
                mouse_y: 0.0,
                mouse_down: false,
                mouse_right: false,
                scroll: 0.0,
                close_requested: false,
                startup_error: None,
            };
        }

        fn open_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
            let attributes = Window::default_attributes().with_title(self.title.clone()).with_inner_size(PhysicalSize::new(self.width, self.height)).with_resizable(false);
            let window = Arc::new(event_loop.create_window(attributes).map_err(|e| format!("game_run: could not open a window: {}", e))?);
            let context = softbuffer::Context::new(window.clone()).map_err(|e| format!("game_run: could not reach the screen: {}", e))?;
            let surface = softbuffer::Surface::new(&context, window.clone()).map_err(|e| format!("game_run: could not reach the screen: {}", e))?;
            self.window = Some(window);
            self.surface = Some(surface);
            return Ok(());
        }
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_none() && self.startup_error.is_none() {
                if let Err(error) = self.open_window(event_loop) {
                    self.startup_error = Some(error);
                }
            }
        }

        fn window_event(&mut self, _event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
            match event {
                WindowEvent::CloseRequested => self.close_requested = true,
                WindowEvent::KeyboardInput { event, .. } => {
                    let name = key_name(&event.logical_key);
                    if name.is_empty() {
                        return;
                    }
                    match event.state {
                        ElementState::Pressed => {
                            if !event.repeat && self.keys_down.insert(name.clone()) {
                                self.keys_pressed.push(name);
                            }
                        }
                        ElementState::Released => {
                            self.keys_down.remove(&name);
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    self.mouse_x = position.x;
                    self.mouse_y = position.y;
                }
                WindowEvent::MouseInput { state, button, .. } => match button {
                    MouseButton::Left => self.mouse_down = state == ElementState::Pressed,
                    MouseButton::Right => self.mouse_right = state == ElementState::Pressed,
                    _ => {}
                },
                WindowEvent::MouseWheel { delta, .. } => {
                    // Lines and pixels arrive on different mice: fold both onto
                    // roughly line-sized units so a game reads one number.
                    self.scroll += match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, vertical) => vertical as f64,
                        winit::event::MouseScrollDelta::PixelDelta(position) => position.y / 40.0,
                    };
                }
                // Losing focus releases every key, otherwise a key held across an
                // alt-tab stays down forever because its release went elsewhere.
                WindowEvent::Focused(false) => self.keys_down.clear(),
                _ => {}
            }
        }
    }

    /// A poll with no input in it, for the frames before the window exists
    /// and for the one that carries the close request.
    fn quiet_poll(close_requested: bool, ready: bool) -> Poll {
        return Poll { close_requested, ready, keys_down: Vec::new(), keys_pressed: Vec::new(), mouse_x: 0.0, mouse_y: 0.0, mouse_down: false, mouse_right: false, scroll: 0.0, touches: Vec::new() };
    }

    /// A note for the person at the terminal, never for captured output.
    fn announce(line: &str) {
        use std::io::IsTerminal;
        if std::io::stderr().is_terminal() {
            eprintln!("{}", line);
        }
    }

    pub struct Backend {
        event_loop: EventLoop<()>,
        app: App,
        epoch: Instant,
        /// The graphics card path, once it comes up. None either before the
        /// first frame or because the machine has nothing to offer, in
        /// which case the CPU rasterizer below carries the game.
        gpu: Option<super::super::game_gpu::GpuRenderer>,
        gpu_settled: bool,
        /// When the next frame is due, an absolute schedule the pacer keeps.
        next_frame_at: Option<Instant>,
    }

    impl Backend {
        /// Makes the event loop and the app that will hold the window. The
        /// window itself arrives later, on winit's first resumed call.
        pub async fn create(config: &GAME_Config, width: u32, height: u32) -> Result<Backend, String> {
            let event_loop = EventLoop::new().map_err(|e| format!("game_run: could not talk to the display - a game needs a desktop to draw on: {}", e))?;
            event_loop.set_control_flow(ControlFlow::Poll);
            let app = App::new(config.title.clone(), width, height);
            return Ok(Backend { event_loop, app, epoch: Instant::now(), gpu: None, gpu_settled: false, next_frame_at: None });
        }

        /// Draws one frame, on the graphics card when there is one and on
        /// the CPU when there is not. The first frame with a window decides
        /// which, and `NAIL_GAME_CPU=1` in the environment forces the CPU
        /// for comparing the two or dodging a broken driver.
        pub async fn render(&mut self, frame: &GAME_Frame, pixmap: &mut tiny_skia::Pixmap, mut overlay: Option<&mut tiny_skia::Pixmap>, pixel_size: u32) -> Result<(), String> {
            if !self.gpu_settled {
                if let Some(window) = self.app.window.clone() {
                    self.gpu_settled = true;
                    if std::env::var("NAIL_GAME_CPU").map(|value| value == "1").unwrap_or(false) {
                        announce("game renderer: the CPU, because NAIL_GAME_CPU is set");
                    } else {
                        let target = super::super::game_gpu::RenderTarget::Window(window.clone());
                        match super::super::game_gpu::GpuRenderer::create(target, self.app.width, self.app.height, pixel_size).await {
                            Ok(mut renderer) => {
                                let real = window.inner_size();
                                renderer.resize_surface(real.width.max(1), real.height.max(1));
                                announce(&format!("game renderer: {}", renderer.describe()));
                                self.gpu = Some(renderer);
                            }
                            Err(super::super::game_gpu::GpuFrameError::Device(why)) | Err(super::super::game_gpu::GpuFrameError::Program(why)) => {
                                announce(&format!("game renderer: the CPU, because {}", why));
                            }
                        }
                    }
                }
            }
            if let Some(gpu) = self.gpu.as_mut() {
                if let Some(window) = self.app.window.as_ref() {
                    let real = window.inner_size();
                    gpu.resize_surface(real.width.max(1), real.height.max(1));
                }
                match gpu.render_frame(frame, pixel_size) {
                    Ok(()) => return Ok(()),
                    Err(super::super::game_gpu::GpuFrameError::Program(why)) => return Err(why),
                    Err(super::super::game_gpu::GpuFrameError::Device(why)) => {
                        // The card died under a running game. Its scenes
                        // went with this half-drawn frame, so skip it and
                        // let the CPU draw from the next one.
                        announce(&format!("game renderer: falling back to the CPU, because {}", why));
                        self.gpu = None;
                        return Ok(());
                    }
                }
            }
            rasterize(pixmap, overlay.as_deref_mut(), frame, pixel_size)?;
            return self.present(pixmap, overlay.as_deref());
        }

        /// Pumps winit once and hands back what the player did. A startup
        /// error from opening the window surfaces here as the Err.
        pub fn poll(&mut self) -> Result<Poll, String> {
            self.event_loop.pump_app_events(Some(Duration::ZERO), &mut self.app);
            if let Some(error) = self.app.startup_error.take() {
                return Err(error);
            }
            if self.app.close_requested {
                return Ok(quiet_poll(true, true));
            }
            if self.app.window.is_none() {
                // The window is created by winit's first resumed call, which
                // on some platforms arrives a few pumps in.
                return Ok(quiet_poll(false, false));
            }
            let mut keys_down: Vec<String> = self.app.keys_down.iter().cloned().collect();
            keys_down.sort();
            return Ok(Poll {
                close_requested: false,
                ready: true,
                keys_down,
                keys_pressed: std::mem::take(&mut self.app.keys_pressed),
                mouse_x: self.app.mouse_x,
                mouse_y: self.app.mouse_y,
                mouse_down: self.app.mouse_down,
                mouse_right: self.app.mouse_right,
                scroll: std::mem::take(&mut self.app.scroll),
                // A desktop has a mouse, not fingers.
                touches: Vec::new(),
            });
        }

        /// Copies the finished pixmap into the window, stretching nearest-neighbour
        /// if the window's real pixel size differs from the game's (a high-DPI screen
        /// does this).
        pub fn present(&mut self, pixmap: &tiny_skia::Pixmap, overlay: Option<&tiny_skia::Pixmap>) -> Result<(), String> {
            let window = self.app.window.as_ref().ok_or_else(|| "game_run: the window disappeared".to_string())?;
            let real = window.inner_size();
            let real_width = real.width.max(1);
            let real_height = real.height.max(1);
            let surface = self.app.surface.as_mut().ok_or_else(|| "game_run: the window disappeared".to_string())?;
            surface
                .resize(NonZeroU32::new(real_width).ok_or_else(|| "game_run: the window has no size".to_string())?, NonZeroU32::new(real_height).ok_or_else(|| "game_run: the window has no size".to_string())?)
                .map_err(|e| format!("game_run: could not size the frame: {}", e))?;

            let mut buffer = surface.buffer_mut().map_err(|e| format!("game_run: could not get the frame to draw into: {}", e))?;
            let source = pixmap.pixels();
            let source_width = pixmap.width() as usize;
            let source_height = pixmap.height() as usize;
            for y in 0..real_height as usize {
                let from_y = (y * source_height / real_height as usize).min(source_height - 1);
                let text_row = overlay.map(|full| {
                    let full_height = full.height() as usize;
                    (y * full_height / real_height as usize).min(full_height - 1)
                });
                for x in 0..real_width as usize {
                    let from_x = (x * source_width / real_width as usize).min(source_width - 1);
                    let pixel = source[from_y * source_width + from_x].demultiply();
                    let mut red = pixel.red() as u32;
                    let mut green = pixel.green() as u32;
                    let mut blue = pixel.blue() as u32;
                    // The full resolution text overlay composites over the
                    // upscaled world, premultiplied source over an opaque
                    // background.
                    if let (Some(full), Some(text_y)) = (overlay, text_row) {
                        let full_width = full.width() as usize;
                        let text_x = (x * full_width / real_width as usize).min(full_width - 1);
                        let ink = full.pixels()[text_y * full_width + text_x];
                        let alpha = ink.alpha() as u32;
                        if alpha > 0 {
                            let keep = 255 - alpha;
                            red = ink.red() as u32 + red * keep / 255;
                            green = ink.green() as u32 + green * keep / 255;
                            blue = ink.blue() as u32 + blue * keep / 255;
                        }
                    }
                    buffer[y * real_width as usize + x] = (red.min(255) << 16) | (green.min(255) << 8) | blue.min(255);
                }
            }
            buffer.present().map_err(|e| format!("game_run: could not put the frame on screen: {}", e))?;
            return Ok(());
        }

        /// Sleeps out whatever the frame budget left over, so the loop lands
        /// on `target_fps`. Waiting is async sleep, so the runtime this
        /// shares a thread with keeps serving whatever else the program spawned.
        pub async fn pace(&mut self, _frame_work_ms: f64, target_fps: i64) -> Result<(), String> {
            if target_fps > 0 {
                let budget = Duration::from_secs_f64(1.0 / target_fps as f64);
                let now = Instant::now();
                // An absolute schedule, not "sleep the leftover": sleeping
                // the leftover adds the sleep's overshoot and the loop's own
                // overhead to every frame, which is how a 60 cap read 56.
                let deadline = match self.next_frame_at {
                    Some(planned) if planned <= now => {
                        // Running behind. Draw immediately and restart the
                        // schedule rather than sleeping or chasing the debt.
                        self.next_frame_at = Some(now + budget);
                        return Ok(());
                    }
                    Some(planned) if planned <= now + budget => planned,
                    _ => now + budget,
                };
                // Sleep to within two milliseconds of the deadline, then
                // spin the rest: the OS wakes a sleeper about a timer tick
                // late, the spin is exact.
                if let Some(coarse) = deadline.checked_sub(Duration::from_millis(2)) {
                    if coarse > now {
                        tokio::time::sleep(coarse - now).await;
                    }
                }
                while Instant::now() < deadline {
                    std::hint::spin_loop();
                }
                self.next_frame_at = Some(deadline + budget);
            } else {
                // Unpaced still has to yield, or a fast game starves the runtime.
                tokio::task::yield_now().await;
            }
            return Ok(());
        }

        /// Monotonic milliseconds since the backend was made.
        pub fn now_ms(&self) -> f64 {
            return self.epoch.elapsed().as_secs_f64() * 1000.0;
        }

        /// The whole-game answer to "is it fast enough": how many frames really
        /// showed per second, and how many it could draw flat out. Printed once
        /// when the game closes, only when a human's terminal is attached, so
        /// captured output never sees it.
        pub fn report_close(&self, frames: u64, work_ms: f64, wall_ms: f64, target_fps: i64) {
            use std::io::IsTerminal;
            if frames == 0 || !std::io::stderr().is_terminal() {
                return;
            }
            let wall = (wall_ms / 1000.0).max(f64::MIN_POSITIVE);
            let actual = frames as f64 / wall;
            let average_work = (work_ms / 1000.0) / frames as f64;
            let possible = 1.0 / average_work.max(f64::MIN_POSITIVE);
            let pacing = if target_fps > 0 { format!(", the rest waiting out the {} fps target", target_fps) } else { String::new() };
            eprintln!("game frame rate: {:.0} fps shown, {:.0} fps possible, {:.2}ms of real work per frame{}", actual, possible, average_work * 1000.0, pacing);
        }

        /// A short nap for the polls before the window exists.
        pub async fn idle_wait(&self) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}

/// The browser backend. The game itself cannot tell the difference: same
/// shapes, same callbacks, same input names. What changes is underneath -
/// the picture goes to a canvas element instead of a window, the keyboard
/// comes from DOM events, and the browser paces the loop with
/// requestAnimationFrame, so `target_fps` is ignored on the web and
/// `delta_ms` is how a game stays speed-correct.
///
/// The canvas is the element with id `nail-game` if the page has one, and a
/// new canvas appended to the body if it does not.
#[cfg(target_arch = "wasm32")]
mod web_backend {
    /// The CSS properties this backend ever sets. A closed choice, so it is
    /// an enum rather than free text at the call sites.
    enum CanvasStyle {
        Width,
        Height,
        TouchAction,
    }

    impl CanvasStyle {
        fn name(&self) -> &'static str {
            match self {
                CanvasStyle::Width => "width",
                CanvasStyle::Height => "height",
                CanvasStyle::TouchAction => "touch-action",
            }
        }
    }

    /// What the browser may do with a finger on the canvas. A game wants the
    /// finger for itself, not for scrolling and zooming the page.
    enum TouchAction {
        Off,
    }

    impl TouchAction {
        fn value(&self) -> &'static str {
            match self {
                TouchAction::Off => "none",
            }
        }
    }

    fn set_style(style: &web_sys::CssStyleDeclaration, property: CanvasStyle, value: &str) {
        let _ = style.set_property(property.name(), value);
    }

    use super::*;
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{Clamped, JsCast};

    /// The word the shared loop's size errors call the drawing surface.
    pub const SURFACE_NOUN: &str = "canvas";

    #[derive(Default)]
    struct WebInput {
        keys_down: HashSet<String>,
        keys_pressed: Vec<String>,
        mouse_x: f64,
        mouse_y: f64,
        mouse_down: bool,
        mouse_right: bool,
        scroll: f64,
        /// How far apart two fingers were last time they both touched, so a
        /// pinch can be turned into scrolling. Zero means no pinch underway.
        pinch_span: f64,
        /// Every finger touching right now, x then y for each. A game with
        /// two controls on screen at once needs all of them, not just the
        /// one that happens to be first.
        touches: Vec<f64>,
    }

    /// The DOM's names for keys, folded onto the same names the native build
    /// uses so a game is portable without knowing it.
    fn web_key_name(key: &str) -> String {
        return match key {
            "ArrowUp" => "Up".to_string(),
            "ArrowDown" => "Down".to_string(),
            "ArrowLeft" => "Left".to_string(),
            "ArrowRight" => "Right".to_string(),
            " " => "Space".to_string(),
            "Enter" => "Enter".to_string(),
            "Escape" => "Esc".to_string(),
            "Shift" => "Shift".to_string(),
            "Control" => "Ctrl".to_string(),
            "Alt" => "Alt".to_string(),
            "Tab" => "Tab".to_string(),
            "Backspace" => "Backspace".to_string(),
            other => {
                if other.chars().count() == 1 {
                    return other.to_lowercase();
                }
                return String::new();
            }
        };
    }

    /// One requestAnimationFrame, awaitable. Resolves to the browser's
    /// timestamp for the frame, though the shared loop keeps its own clock.
    async fn next_frame(window: &web_sys::Window) -> Result<f64, String> {
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            let _ = window.request_animation_frame(&resolve);
        });
        let stamp = wasm_bindgen_futures::JsFuture::from(promise).await.map_err(|_| "game_run: the browser stopped animating".to_string())?;
        return Ok(stamp.as_f64().unwrap_or(0.0));
    }

    type KeyClosure = Closure<dyn FnMut(web_sys::KeyboardEvent)>;
    type MouseClosure = Closure<dyn FnMut(web_sys::MouseEvent)>;
    type WheelClosure = Closure<dyn FnMut(web_sys::WheelEvent)>;
    type TouchClosure = Closure<dyn FnMut(web_sys::TouchEvent)>;

    /// Where a finger is in the game's own coordinates. The canvas may be
    /// displayed at any size the page likes, so the position inside its
    /// bounding box is scaled back to the size the game thinks it has.
    fn touch_position(canvas: &web_sys::HtmlCanvasElement, touch: &web_sys::Touch, game_width: f64, game_height: f64) -> (f64, f64) {
        let box_on_screen = canvas.get_bounding_client_rect();
        let shown_width = box_on_screen.width().max(1.0);
        let shown_height = box_on_screen.height().max(1.0);
        let across = (touch.client_x() as f64 - box_on_screen.left()) * game_width / shown_width;
        let down = (touch.client_y() as f64 - box_on_screen.top()) * game_height / shown_height;
        return (across, down);
    }

    /// Every finger on the canvas, as x then y for each, in game
    /// coordinates. A game reads this when it has more than one control on
    /// screen and a player may hold two of them at once.
    fn touch_points(canvas: &web_sys::HtmlCanvasElement, touches: &web_sys::TouchList, game_width: f64, game_height: f64) -> Vec<f64> {
        let mut points = Vec::new();
        for index in 0..touches.length() {
            let Some(finger) = touches.get(index) else { continue };
            let (across, down) = touch_position(canvas, &finger, game_width, game_height);
            points.push(across);
            points.push(down);
        }
        return points;
    }

    /// How far apart the first two fingers are, or zero when fewer than two
    /// are touching.
    fn touch_span(touches: &web_sys::TouchList) -> f64 {
        let (Some(first), Some(second)) = (touches.get(0), touches.get(1)) else { return 0.0 };
        let across = first.client_x() as f64 - second.client_x() as f64;
        let down = first.client_y() as f64 - second.client_y() as f64;
        return (across * across + down * down).sqrt();
    }

    fn listen(target: &web_sys::EventTarget, name: &str, callback: &wasm_bindgen::JsValue) {
        let _ = target.add_event_listener_with_callback(name, callback.dyn_ref().unwrap());
    }

    fn unlisten(target: &web_sys::EventTarget, name: &str, callback: &wasm_bindgen::JsValue) {
        let _ = target.remove_event_listener_with_callback(name, callback.dyn_ref().unwrap());
    }

    pub struct Backend {
        window: web_sys::Window,
        canvas: web_sys::HtmlCanvasElement,
        /// The 2d context, taken only when the CPU path actually draws. A
        /// canvas hands out one kind of context in its life, so the card
        /// path must be given its chance before anything touches 2d.
        context: Option<web_sys::CanvasRenderingContext2d>,
        gpu: Option<super::super::game_gpu::GpuRenderer>,
        gpu_settled: bool,
        input: Rc<RefCell<WebInput>>,
        straight: Vec<u8>,
        width: u32,
        height: u32,
        keydown: KeyClosure,
        keyup: KeyClosure,
        mousemove: MouseClosure,
        mousedown: MouseClosure,
        mouseup: MouseClosure,
        wheel: WheelClosure,
        touch_start: TouchClosure,
        touch_move: TouchClosure,
        touch_end: TouchClosure,
    }

    impl Backend {
        /// Finds or makes the canvas, wires up the DOM listeners, and lets
        /// one animation frame pass so the loop starts on the browser's own
        /// cadence.
        pub async fn create(config: &GAME_Config, width: u32, height: u32) -> Result<Backend, String> {
            let window = web_sys::window().ok_or_else(|| "game_run: there is no browser window to draw in".to_string())?;
            let document = window.document().ok_or_else(|| "game_run: the page has no document".to_string())?;
            let canvas: web_sys::HtmlCanvasElement = match document.get_element_by_id("nail-game") {
                Some(element) => element.dyn_into().map_err(|_| "game_run: the element with id nail-game is not a canvas".to_string())?,
                None => {
                    let element = document.create_element("canvas").map_err(|_| "game_run: could not make a canvas".to_string())?;
                    element.set_id("nail-game");
                    document.body().ok_or_else(|| "game_run: the page has no body to put a canvas in".to_string())?.append_child(&element).map_err(|_| "game_run: could not add the canvas to the page".to_string())?;
                    element.dyn_into().map_err(|_| "game_run: could not make a canvas".to_string())?
                }
            };
            // The canvas holds the small chunky-pixel buffer, CSS stretches it
            // back to the configured size, and pixelated keeps the edges crisp
            // instead of smeared.
            let pixel_size = pixel_size_of(config);
            let buffer_width = (width / pixel_size).max(1);
            let buffer_height = (height / pixel_size).max(1);
            // With chunky pixels the world buffer is small but text renders
            // full size on an overlay, so the canvas keeps full resolution
            // and present upscales the world in software before compositing.
            let composite = pixel_size > 1;
            let canvas_width = if composite { width } else { buffer_width };
            let canvas_height = if composite { height } else { buffer_height };
            canvas.set_width(canvas_width);
            canvas.set_height(canvas_height);
            let style = canvas.style();
            set_style(&style, CanvasStyle::Width, &format!("{}px", width));
            set_style(&style, CanvasStyle::Height, &format!("{}px", height));
            set_style(&style, CanvasStyle::TouchAction, TouchAction::Off.value());

            let input = Rc::new(RefCell::new(WebInput::default()));

            let keydown = {
                let input = input.clone();
                Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
                    let name = web_key_name(&event.key());
                    if name.is_empty() {
                        return;
                    }
                    // Arrows and space scroll the page otherwise, which makes a
                    // game unplayable inside any page tall enough to scroll.
                    if matches!(name.as_str(), "Up" | "Down" | "Left" | "Right" | "Space" | "Tab" | "Backspace") {
                        event.prevent_default();
                    }
                    let mut state = input.borrow_mut();
                    if !event.repeat() && state.keys_down.insert(name.clone()) {
                        state.keys_pressed.push(name);
                    }
                })
            };
            let keyup = {
                let input = input.clone();
                Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
                    let name = web_key_name(&event.key());
                    if !name.is_empty() {
                        input.borrow_mut().keys_down.remove(&name);
                    }
                })
            };
            let mousemove = {
                let input = input.clone();
                Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
                    let mut state = input.borrow_mut();
                    state.mouse_x = event.offset_x() as f64;
                    state.mouse_y = event.offset_y() as f64;
                })
            };
            let mousedown = {
                let input = input.clone();
                Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
                    let mut state = input.borrow_mut();
                    match event.button() {
                        0 => state.mouse_down = true,
                        2 => state.mouse_right = true,
                        _ => {}
                    }
                })
            };
            let mouseup = {
                let input = input.clone();
                Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
                    let mut state = input.borrow_mut();
                    match event.button() {
                        0 => state.mouse_down = false,
                        2 => state.mouse_right = false,
                        _ => {}
                    }
                })
            };
            let wheel = {
                let input = input.clone();
                Closure::<dyn FnMut(web_sys::WheelEvent)>::new(move |event: web_sys::WheelEvent| {
                    event.prevent_default();
                    input.borrow_mut().scroll += event.delta_y() / -100.0;
                })
            };
            // Fingers arrive as the same press and drag a mouse would make,
            // so a game that reads the mouse is playable on a phone without
            // knowing touch exists. Two fingers pinching feed the scroll.
            let touch_start = {
                let input = input.clone();
                let canvas = canvas.clone();
                let game_width = width as f64;
                let game_height = height as f64;
                Closure::<dyn FnMut(web_sys::TouchEvent)>::new(move |event: web_sys::TouchEvent| {
                    event.prevent_default();
                    let touches = event.touches();
                    let mut state = input.borrow_mut();
                    if let Some(finger) = touches.get(0) {
                        let (across, down) = touch_position(&canvas, &finger, game_width, game_height);
                        state.mouse_x = across;
                        state.mouse_y = down;
                        state.mouse_down = true;
                    }
                    state.touches = touch_points(&canvas, &touches, game_width, game_height);
                    state.pinch_span = touch_span(&touches);
                })
            };
            let touch_move = {
                let input = input.clone();
                let canvas = canvas.clone();
                let game_width = width as f64;
                let game_height = height as f64;
                Closure::<dyn FnMut(web_sys::TouchEvent)>::new(move |event: web_sys::TouchEvent| {
                    event.prevent_default();
                    let touches = event.touches();
                    let mut state = input.borrow_mut();
                    if let Some(finger) = touches.get(0) {
                        let (across, down) = touch_position(&canvas, &finger, game_width, game_height);
                        state.mouse_x = across;
                        state.mouse_y = down;
                    }
                    state.touches = touch_points(&canvas, &touches, game_width, game_height);
                    // Spreading two fingers scrolls the same way a wheel
                    // scrolls up, which is what zooming in means to a game.
                    let span = touch_span(&touches);
                    if span > 0.0 && state.pinch_span > 0.0 {
                        state.scroll += (span - state.pinch_span) / 50.0;
                    }
                    state.pinch_span = span;
                })
            };
            let touch_end = {
                let input = input.clone();
                let canvas = canvas.clone();
                let game_width = width as f64;
                let game_height = height as f64;
                Closure::<dyn FnMut(web_sys::TouchEvent)>::new(move |event: web_sys::TouchEvent| {
                    event.prevent_default();
                    let touches = event.touches();
                    let mut state = input.borrow_mut();
                    state.touches = touch_points(&canvas, &touches, game_width, game_height);
                    state.pinch_span = touch_span(&touches);
                    if touches.length() == 0 {
                        state.mouse_down = false;
                    }
                })
            };
            listen(&window, "keydown", keydown.as_ref());
            listen(&window, "keyup", keyup.as_ref());
            listen(canvas.as_ref(), "mousemove", mousemove.as_ref());
            listen(canvas.as_ref(), "mousedown", mousedown.as_ref());
            listen(canvas.as_ref(), "mouseup", mouseup.as_ref());
            listen(canvas.as_ref(), "wheel", wheel.as_ref());
            listen(canvas.as_ref(), "touchstart", touch_start.as_ref());
            listen(canvas.as_ref(), "touchmove", touch_move.as_ref());
            listen(canvas.as_ref(), "touchend", touch_end.as_ref());
            listen(canvas.as_ref(), "touchcancel", touch_end.as_ref());

            let straight = vec![0u8; canvas_width as usize * canvas_height as usize * 4];

            next_frame(&window).await?;

            return Ok(Backend { window, canvas, context: None, gpu: None, gpu_settled: false, input, straight, width, height, keydown, keyup, mousemove, mousedown, mouseup, wheel, touch_start, touch_move, touch_end });
        }

        /// Draws one frame, on the graphics card through WebGL2 when the
        /// browser offers it, on the canvas's 2d context when it does not.
        pub async fn render(&mut self, frame: &GAME_Frame, pixmap: &mut tiny_skia::Pixmap, mut overlay: Option<&mut tiny_skia::Pixmap>, pixel_size: u32) -> Result<(), String> {
            if !self.gpu_settled {
                self.gpu_settled = true;
                match self.try_gpu(pixel_size).await {
                    Ok(renderer) => {
                        web_sys::console::log_1(&format!("nail game renderer: {}", renderer.describe()).into());
                        self.gpu = Some(renderer);
                    }
                    Err(why) => {
                        web_sys::console::log_1(&format!("nail game renderer: the CPU, because {}", why).into());
                    }
                }
            }
            if let Some(gpu) = self.gpu.as_mut() {
                return match gpu.render_frame(frame, pixel_size) {
                    Ok(()) => Ok(()),
                    Err(super::super::game_gpu::GpuFrameError::Program(why)) => Err(why),
                    // The canvas gave its one context to WebGL and cannot
                    // hand out a 2d one now, so a dead context ends the
                    // game honestly instead of showing a frozen picture.
                    Err(super::super::game_gpu::GpuFrameError::Device(why)) => Err(format!("game_run: the browser's graphics context failed: {}", why)),
                };
            }
            rasterize(pixmap, overlay.as_deref_mut(), frame, pixel_size)?;
            return self.present(pixmap, overlay.as_deref());
        }

        /// Tries the whole card path on a throwaway canvas first. Only when
        /// the browser proves it can do WebGL2 end to end does the real
        /// canvas commit to it - a canvas only ever gives out one kind of
        /// context, and a failed try must leave 2d available.
        async fn try_gpu(&self, pixel_size: u32) -> Result<super::super::game_gpu::GpuRenderer, String> {
            let describe = |problem: super::super::game_gpu::GpuFrameError| match problem {
                super::super::game_gpu::GpuFrameError::Device(why) | super::super::game_gpu::GpuFrameError::Program(why) => why,
            };
            let document = self.window.document().ok_or_else(|| "the page has no document".to_string())?;
            let probe: web_sys::HtmlCanvasElement = document
                .create_element("canvas")
                .ok()
                .and_then(|element| element.dyn_into().ok())
                .ok_or_else(|| "could not make a canvas to probe with".to_string())?;
            super::super::game_gpu::GpuRenderer::create(super::super::game_gpu::RenderTarget::Canvas(probe), self.width, self.height, pixel_size).await.map_err(describe)?;
            return super::super::game_gpu::GpuRenderer::create(super::super::game_gpu::RenderTarget::Canvas(self.canvas.clone()), self.width, self.height, pixel_size).await.map_err(describe);
        }

        /// Hands back what the DOM listeners collected since the last poll.
        /// The canvas is always ready and a page has no close button.
        pub fn poll(&mut self) -> Result<Poll, String> {
            let mut pending = self.input.borrow_mut();
            let mut keys_down: Vec<String> = pending.keys_down.iter().cloned().collect();
            keys_down.sort();
            return Ok(Poll {
                close_requested: false,
                ready: true,
                keys_down,
                keys_pressed: std::mem::take(&mut pending.keys_pressed),
                mouse_x: pending.mouse_x,
                mouse_y: pending.mouse_y,
                mouse_down: pending.mouse_down,
                mouse_right: pending.mouse_right,
                scroll: std::mem::take(&mut pending.scroll),
                touches: pending.touches.clone(),
            });
        }

        /// Copies the finished pixmap onto the canvas. With an overlay the
        /// world upscales in software and the full resolution text
        /// composites over it, without one the pixmap copies straight in.
        pub fn present(&mut self, pixmap: &tiny_skia::Pixmap, overlay: Option<&tiny_skia::Pixmap>) -> Result<(), String> {
            let (image_width, image_height) = match overlay {
                Some(full) => {
                    let out_width = full.width() as usize;
                    let out_height = full.height() as usize;
                    let source = pixmap.pixels();
                    let source_width = pixmap.width() as usize;
                    let source_height = pixmap.height() as usize;
                    let ink_pixels = full.pixels();
                    for y in 0..out_height {
                        let from_y = (y * source_height / out_height).min(source_height - 1);
                        for x in 0..out_width {
                            let from_x = (x * source_width / out_width).min(source_width - 1);
                            let world = source[from_y * source_width + from_x].demultiply();
                            let mut red = world.red() as u32;
                            let mut green = world.green() as u32;
                            let mut blue = world.blue() as u32;
                            let ink = ink_pixels[y * out_width + x];
                            let alpha = ink.alpha() as u32;
                            if alpha > 0 {
                                let keep = 255 - alpha;
                                red = ink.red() as u32 + red * keep / 255;
                                green = ink.green() as u32 + green * keep / 255;
                                blue = ink.blue() as u32 + blue * keep / 255;
                            }
                            let index = (y * out_width + x) * 4;
                            self.straight[index] = red.min(255) as u8;
                            self.straight[index + 1] = green.min(255) as u8;
                            self.straight[index + 2] = blue.min(255) as u8;
                            self.straight[index + 3] = 255;
                        }
                    }
                    (full.width(), full.height())
                }
                None => {
                    // The canvas wants straight alpha, the pixmap holds
                    // premultiplied.
                    for (index, pixel) in pixmap.pixels().iter().enumerate() {
                        let color = pixel.demultiply();
                        self.straight[index * 4] = color.red();
                        self.straight[index * 4 + 1] = color.green();
                        self.straight[index * 4 + 2] = color.blue();
                        self.straight[index * 4 + 3] = color.alpha();
                    }
                    (pixmap.width(), pixmap.height())
                }
            };
            if self.context.is_none() {
                let context: web_sys::CanvasRenderingContext2d = self
                    .canvas
                    .get_context("2d")
                    .ok()
                    .flatten()
                    .ok_or_else(|| "game_run: the canvas would not give a 2d context".to_string())?
                    .dyn_into()
                    .map_err(|_| "game_run: the canvas would not give a 2d context".to_string())?;
                self.context = Some(context);
            }
            let context = self.context.as_ref().expect("just filled");
            let image = web_sys::ImageData::new_with_u8_clamped_array_and_sh(Clamped(&self.straight), image_width, image_height).map_err(|_| "game_run: could not build the frame image".to_string())?;
            if context.put_image_data(&image, 0.0, 0.0).is_err() {
                return Err("game_run: could not put the frame on the canvas".to_string());
            }
            return Ok(());
        }

        /// Awaits the next requestAnimationFrame. The browser decides the
        /// cadence, so the frame budget and `target_fps` have nothing to say.
        pub async fn pace(&mut self, _frame_work_ms: f64, _target_fps: i64) -> Result<(), String> {
            next_frame(&self.window).await?;
            return Ok(());
        }

        /// Monotonic milliseconds from the browser's performance clock.
        pub fn now_ms(&self) -> f64 {
            return match self.window.performance() {
                Some(performance) => performance.now(),
                None => js_sys::Date::now(),
            };
        }

        /// A browser tab has no terminal to print a closing line to.
        pub fn report_close(&self, _frames: u64, _work_ms: f64, _wall_ms: f64, _target_fps: i64) {}

        /// Waits a frame. The canvas never reports not ready, so this only
        /// exists to complete the backend surface.
        pub async fn idle_wait(&self) {
            let _ = next_frame(&self.window).await;
        }
    }

    impl Drop for Backend {
        fn drop(&mut self) {
            unlisten(&self.window, "keydown", self.keydown.as_ref());
            unlisten(&self.window, "keyup", self.keyup.as_ref());
            unlisten(self.canvas.as_ref(), "mousemove", self.mousemove.as_ref());
            unlisten(self.canvas.as_ref(), "mousedown", self.mousedown.as_ref());
            unlisten(self.canvas.as_ref(), "mouseup", self.mouseup.as_ref());
            unlisten(self.canvas.as_ref(), "wheel", self.wheel.as_ref());
            unlisten(self.canvas.as_ref(), "touchstart", self.touch_start.as_ref());
            unlisten(self.canvas.as_ref(), "touchmove", self.touch_move.as_ref());
            unlisten(self.canvas.as_ref(), "touchend", self.touch_end.as_ref());
            unlisten(self.canvas.as_ref(), "touchcancel", self.touch_end.as_ref());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_fill_only_their_own_fields() {
        let shape = rect(1.0, 2.0, 3.0, 4.0, "red".to_string());
        assert_eq!(shape.kind, "rect");
        assert_eq!(shape.width, 3.0);
        assert_eq!(shape.radius, 0.0);

        let shape = circle(5.0, 6.0, 7.0, "#fff".to_string());
        assert_eq!(shape.kind, "circle");
        assert_eq!(shape.radius, 7.0);
        assert_eq!(shape.width, 0.0);

        let shape = text("score".to_string(), 8.0, 9.0, 24.0, "white".to_string());
        assert_eq!(shape.kind, "text");
        assert_eq!(shape.text, "score");
        assert_eq!(shape.size, 24.0);
    }

    #[test]
    fn colors_parse_in_every_written_form() {
        assert!(parse_color("#ff0000").is_ok());
        assert!(parse_color("#f00").is_ok());
        assert!(parse_color("#ff000080").is_ok());
        assert!(parse_color("red").is_ok());
        assert!(parse_color("nonsense").is_err());
        assert!(parse_color("#ff00").is_err());
        assert!(parse_color("#gg0000").is_err());
    }

    #[test]
    fn the_built_in_font_parses() {
        assert!(font().is_ok());
    }

    #[test]
    fn a_bad_shape_kind_is_an_error_not_a_blank_frame() {
        let mut pixmap = tiny_skia::Pixmap::new(10, 10).unwrap();
        let mut shape = blank("nonsense");
        shape.color = "red".to_string();
        let frame = GAME_Frame { shapes: vec![shape], background: "black".to_string(), quit: false };
        assert!(rasterize(&mut pixmap, None, &frame, 1).is_err());
    }

    #[test]
    fn rasterizing_a_frame_paints_the_background() {
        let mut pixmap = tiny_skia::Pixmap::new(4, 4).unwrap();
        let frame = GAME_Frame { shapes: vec![rect(1.0, 1.0, 2.0, 2.0, "white".to_string())], background: "#000000".to_string(), quit: false };
        rasterize(&mut pixmap, None, &frame, 1).unwrap();
        let corner = pixmap.pixels()[0].demultiply();
        assert_eq!((corner.red(), corner.green(), corner.blue()), (0, 0, 0));
        let middle = pixmap.pixels()[1 * 4 + 1].demultiply();
        assert_eq!((middle.red(), middle.green(), middle.blue()), (255, 255, 255));
    }
}
