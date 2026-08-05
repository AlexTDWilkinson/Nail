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
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Window, WindowId};

/// How the window starts out. A `target_fps` of 0 means unpaced - the loop
/// runs as fast as update and view come back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAME_Config {
    pub title: String,
    pub width: i64,
    pub height: i64,
    pub target_fps: i64,
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
    pub delta_ms: f64,
}

fn blank(kind: &str) -> GAME_Shape {
    return GAME_Shape {
        kind: kind.to_string(),
        x_coordinate: 0.0,
        y_coordinate: 0.0,
        width: 0.0,
        height: 0.0,
        end_x: 0.0,
        end_y: 0.0,
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
pub fn sprite_load(path: String) -> Result<i64, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("game_sprite_load: could not read {}: {}", path, e))?;
    let pixmap = tiny_skia::Pixmap::decode_png(&bytes).map_err(|e| format!("game_sprite_load: {} is not a PNG this understands: {}", path, e))?;
    let handle = NEXT_SPRITE.fetch_add(1, Ordering::Relaxed);
    sprites().lock().map_err(|_| "game_sprite_load: the sprite store is poisoned".to_string())?.insert(handle, pixmap);
    return Ok(handle);
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
fn parse_color(name: &str) -> Result<tiny_skia::Color, String> {
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
            // Losing focus releases every key, otherwise a key held across an
            // alt-tab stays down forever because its release went elsewhere.
            WindowEvent::Focused(false) => self.keys_down.clear(),
            _ => {}
        }
    }
}

/// Paints one frame's shapes into the pixmap.
fn rasterize(pixmap: &mut tiny_skia::Pixmap, frame: &GAME_Frame) -> Result<(), String> {
    pixmap.fill(parse_color(&frame.background)?);

    for shape in &frame.shapes {
        let identity = tiny_skia::Transform::identity();
        match shape.kind.as_str() {
            "rect" => {
                let Some(rect) = tiny_skia::Rect::from_xywh(shape.x_coordinate as f32, shape.y_coordinate as f32, shape.width as f32, shape.height as f32) else { continue };
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(parse_color(&shape.color)?);
                paint.anti_alias = true;
                pixmap.fill_rect(rect, &paint, identity, None);
            }
            "rect_outline" => {
                let Some(rect) = tiny_skia::Rect::from_xywh(shape.x_coordinate as f32, shape.y_coordinate as f32, shape.width as f32, shape.height as f32) else { continue };
                let path = tiny_skia::PathBuilder::from_rect(rect);
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(parse_color(&shape.color)?);
                paint.anti_alias = true;
                let stroke = tiny_skia::Stroke { width: shape.thickness as f32, ..tiny_skia::Stroke::default() };
                pixmap.stroke_path(&path, &paint, &stroke, identity, None);
            }
            "circle" => {
                let Some(path) = tiny_skia::PathBuilder::from_circle(shape.x_coordinate as f32, shape.y_coordinate as f32, shape.radius as f32) else { continue };
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(parse_color(&shape.color)?);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, identity, None);
            }
            "line" => {
                let mut builder = tiny_skia::PathBuilder::new();
                builder.move_to(shape.x_coordinate as f32, shape.y_coordinate as f32);
                builder.line_to(shape.end_x as f32, shape.end_y as f32);
                let Some(path) = builder.finish() else { continue };
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(parse_color(&shape.color)?);
                paint.anti_alias = true;
                let stroke = tiny_skia::Stroke { width: shape.thickness as f32, ..tiny_skia::Stroke::default() };
                pixmap.stroke_path(&path, &paint, &stroke, identity, None);
            }
            "text" => {
                draw_text(pixmap, shape)?;
            }
            "sprite" | "sprite_scaled" => {
                let store = sprites().lock().map_err(|_| "game_run: the sprite store is poisoned".to_string())?;
                let Some(loaded) = store.get(&shape.sprite) else {
                    return Err(format!("game_run: shape refers to sprite {} but no sprite with that number was loaded", shape.sprite));
                };
                let transform = if shape.kind == "sprite_scaled" && loaded.width() > 0 && loaded.height() > 0 {
                    let scale_x = shape.width as f32 / loaded.width() as f32;
                    let scale_y = shape.height as f32 / loaded.height() as f32;
                    tiny_skia::Transform::from_scale(scale_x, scale_y).post_translate(shape.x_coordinate as f32, shape.y_coordinate as f32)
                } else {
                    tiny_skia::Transform::from_translate(shape.x_coordinate as f32, shape.y_coordinate as f32)
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
fn draw_text(pixmap: &mut tiny_skia::Pixmap, shape: &GAME_Shape) -> Result<(), String> {
    let font = font()?;
    let color = parse_color(&shape.color)?;
    let red = (color.red() * 255.0) as u16;
    let green = (color.green() * 255.0) as u16;
    let blue = (color.blue() * 255.0) as u16;
    let size = shape.size as f32;
    // The shape's y is the top of the text, glyphs hang from the baseline
    // below it. The size itself is a workable ascent for one line.
    let baseline = shape.y_coordinate as f32 + size;
    let mut cursor = shape.x_coordinate as f32;
    let pixmap_width = pixmap.width() as i32;
    let pixmap_height = pixmap.height() as i32;

    for character in shape.text.chars() {
        let (metrics, coverage) = font.rasterize(character, size);
        let glyph_left = cursor as i32 + metrics.xmin;
        let glyph_top = baseline as i32 - metrics.height as i32 - metrics.ymin;
        let data = pixmap.data_mut();
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let alpha = coverage[row * metrics.width + column] as u16;
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

/// Copies the finished pixmap into the window, stretching nearest-neighbour
/// if the window's real pixel size differs from the game's (a high-DPI screen
/// does this).
fn present(app: &mut App, pixmap: &tiny_skia::Pixmap) -> Result<(), String> {
    let window = app.window.as_ref().ok_or_else(|| "game_run: the window disappeared".to_string())?;
    let real = window.inner_size();
    let real_width = real.width.max(1);
    let real_height = real.height.max(1);
    let surface = app.surface.as_mut().ok_or_else(|| "game_run: the window disappeared".to_string())?;
    surface
        .resize(NonZeroU32::new(real_width).ok_or_else(|| "game_run: the window has no size".to_string())?, NonZeroU32::new(real_height).ok_or_else(|| "game_run: the window has no size".to_string())?)
        .map_err(|e| format!("game_run: could not size the frame: {}", e))?;

    let mut buffer = surface.buffer_mut().map_err(|e| format!("game_run: could not get the frame to draw into: {}", e))?;
    let source = pixmap.pixels();
    let source_width = pixmap.width() as usize;
    let source_height = pixmap.height() as usize;
    for y in 0..real_height as usize {
        let from_y = (y * source_height / real_height as usize).min(source_height - 1);
        for x in 0..real_width as usize {
            let from_x = (x * source_width / real_width as usize).min(source_width - 1);
            let pixel = source[from_y * source_width + from_x].demultiply();
            buffer[y * real_width as usize + x] = ((pixel.red() as u32) << 16) | ((pixel.green() as u32) << 8) | pixel.blue() as u32;
        }
    }
    buffer.present().map_err(|e| format!("game_run: could not put the frame on screen: {}", e))?;
    return Ok(());
}

pub type ViewFuture = Pin<Box<dyn Future<Output = GAME_Frame> + Send>>;
pub type UpdateFuture<S> = Pin<Box<dyn Future<Output = S> + Send>>;

/// Opens the window and runs the game until its view reports `quit` or the
/// player closes the window, and returns the state it finished with.
///
/// The loop is: hand the player's input to `update`, draw what `view` says,
/// wait out the rest of the frame, repeat. Waiting is async sleep, so the
/// runtime this shares a thread with keeps serving anything else the program
/// spawned.
pub async fn run<S, V, U>(config: GAME_Config, initial: S, view: V, update: U) -> Result<S, String>
where
    S: Clone + Send + 'static,
    V: Fn(S) -> ViewFuture + Send + Sync + 'static,
    U: Fn(S, GAME_Input) -> UpdateFuture<S> + Send + Sync + 'static,
{
    let width = u32::try_from(config.width).ok().filter(|size| *size > 0).ok_or_else(|| format!("game_run: {} is not a width a window can have", config.width))?;
    let height = u32::try_from(config.height).ok().filter(|size| *size > 0).ok_or_else(|| format!("game_run: {} is not a height a window can have", config.height))?;

    let mut event_loop = EventLoop::new().map_err(|e| format!("game_run: could not talk to the display - a game needs a desktop to draw on: {}", e))?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(config.title.clone(), width, height);
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| "game_run: could not make the frame".to_string())?;

    let mut state = initial;
    let mut last_frame = Instant::now();
    let frame_budget = if config.target_fps > 0 { Some(Duration::from_secs_f64(1.0 / config.target_fps as f64)) } else { None };

    loop {
        event_loop.pump_app_events(Some(Duration::ZERO), &mut app);
        if let Some(error) = app.startup_error.take() {
            return Err(error);
        }
        if app.close_requested {
            return Ok(state);
        }
        if app.window.is_none() {
            // The window is created by winit's first resumed call, which on
            // some platforms arrives a few pumps in.
            tokio::time::sleep(Duration::from_millis(5)).await;
            continue;
        }

        let now = Instant::now();
        let delta_ms = now.duration_since(last_frame).as_secs_f64() * 1000.0;
        last_frame = now;

        let mut keys_down: Vec<String> = app.keys_down.iter().cloned().collect();
        keys_down.sort();
        let input = GAME_Input {
            keys_down,
            keys_pressed: std::mem::take(&mut app.keys_pressed),
            mouse_x: app.mouse_x,
            mouse_y: app.mouse_y,
            mouse_down: app.mouse_down,
            mouse_right: app.mouse_right,
            delta_ms,
        };

        state = update(state, input).await;
        let frame = view(state.clone()).await;
        rasterize(&mut pixmap, &frame)?;
        present(&mut app, &pixmap)?;
        if frame.quit {
            return Ok(state);
        }

        match frame_budget {
            Some(budget) => {
                let used = last_frame.elapsed();
                if used < budget {
                    tokio::time::sleep(budget - used).await;
                }
            }
            // Unpaced still has to yield, or a fast game starves the runtime.
            None => tokio::task::yield_now().await,
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
        assert!(rasterize(&mut pixmap, &frame).is_err());
    }

    #[test]
    fn rasterizing_a_frame_paints_the_background() {
        let mut pixmap = tiny_skia::Pixmap::new(4, 4).unwrap();
        let frame = GAME_Frame { shapes: vec![rect(1.0, 1.0, 2.0, 2.0, "white".to_string())], background: "#000000".to_string(), quit: false };
        rasterize(&mut pixmap, &frame).unwrap();
        let corner = pixmap.pixels()[0].demultiply();
        assert_eq!((corner.red(), corner.green(), corner.blue()), (0, 0, 0));
        let middle = pixmap.pixels()[1 * 4 + 1].demultiply();
        assert_eq!((middle.red(), middle.green(), middle.blue()), (255, 255, 255));
    }
}
