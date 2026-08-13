//! The graphics card path for `game_run`, shared by the desktop and the
//! browser.
//!
//! The contract with the rest of the game module is one function: hand
//! `render_frame` a `GAME_Frame` and it puts the frame on screen. Shapes
//! draw in array order exactly as the CPU path draws them. 2D shapes are
//! instanced quads - circles and outlines carve their edges with signed
//! distance in the fragment shader, sprites and glyphs are textured, and
//! painter's order is simply draw order - while every `scene3d` shape
//! between them renders as real geometry with a depth buffer, instanced
//! per mesh, lit and fogged in the shader.
//!
//! Everything lands first on an offscreen picture sized to the game's own
//! resolution (divided by `pixel_size`, which is what makes chunky-pixel
//! games look the way they do), and a final pass stretches that onto the
//! window nearest-neighbour. Text keeps full resolution when the world is
//! chunky, drawn straight onto the window over the stretched picture - the
//! same compositing the CPU path does in software with its overlay.
//!
//! Failures split two ways on purpose. A program mistake (a bad tint, a
//! draw naming a mesh that was never loaded) is `Program` and ends the game
//! exactly as it would have on the CPU path. A machine problem (no adapter,
//! a lost surface) is `Device` and the caller falls back to the CPU
//! rasterizer instead of killing a working game.

use std::borrow::Cow;
use std::collections::HashMap;

use super::game::{GAME_Frame, GAME_Shape};
use super::game3d;

/// Why a frame could not be drawn: because the program asked for something
/// impossible, or because the machine stopped cooperating.
pub(crate) enum GpuFrameError {
    Program(String),
    Device(String),
}

/// The near and far planes of the depth buffer. Near matches the CPU
/// projector's cutoff so both paths agree about what is visible, far is
/// simply distant - with a floating point depth buffer there is no
/// precision worth reclaiming by tuning it.
const FAR_PLANE: f64 = 1000.0;

/// Uniform data per scene: a view-projection matrix and five vec4s, padded
/// to the dynamic-offset alignment the device demands.
const UNIFORM_FLOATS: usize = 36;

/// Instance data per 3D draw: a model matrix, three padded columns of the
/// normal matrix, the tint, and the shader params.
const INSTANCE_FLOATS: usize = 36;

/// Instance data per 2D shape: five vec4s - kind and origin, the two axes
/// of its quad, the texture window, the premultiplied colour, and the
/// signed-distance parameters.
const SHAPE2D_FLOATS: usize = 20;

/// The kinds the 2D shader switches on. Everything that is just a filled
/// quad - rects, lines, sprites, glyphs - is `FILL`, and only shapes that
/// carve their edge per pixel get their own number.
const KIND_FILL: f32 = 0.0;
const KIND_CIRCLE: f32 = 1.0;
const KIND_OUTLINE: f32 = 2.0;
const KIND_TRIANGLE: f32 = 3.0;

/// The square texture solid colours, glyphs and the white pixel share.
const ATLAS_SIZE: u32 = 1024;

/// The part of the mesh WGSL every fragment shader shares: the scene
/// uniform, the vertex stage, and the `NAIL_Surface` a custom shader is
/// handed. A custom module is this, then the program's WGSL, then the
/// wrapper that calls its `shade`.
const MESH_SHADER_COMMON: &str = r#"
struct SceneUniform {
    view_projection: mat4x4<f32>,
    // light direction toward the source in xyz, ambient floor in w
    light: vec4<f32>,
    light_color: vec4<f32>,
    // fog colour in rgb, and whether there is fog at all in w
    fog_color: vec4<f32>,
    // fog near in x, fog far in y
    fog_range: vec4<f32>,
    // where the camera stands in xyz, and the clock in seconds in w
    eye: vec4<f32>,
};
@group(0) @binding(0) var<uniform> scene: SceneUniform;

// What a custom shader is handed for every pixel: the surface's own colour
// with any tint already mixed in, its normal, where it sits in the world,
// the direction back toward the camera, the scene's light direction and
// colour, the ambient floor, the clock in seconds, and the draw's glow.
// The shader's one job is turning this into a colour.
struct NAIL_Surface {
    color: vec3<f32>,
    normal: vec3<f32>,
    world_position: vec3<f32>,
    toward_eye: vec3<f32>,
    light: vec3<f32>,
    light_color: vec3<f32>,
    ambient: f32,
    time: f32,
    glow: f32,
    // the draw's own four numbers, set by game3d_draw_shader_params
    params: vec4<f32>,
};

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) model_0: vec4<f32>,
    @location(4) model_1: vec4<f32>,
    @location(5) model_2: vec4<f32>,
    @location(6) model_3: vec4<f32>,
    @location(7) normal_0: vec4<f32>,
    @location(8) normal_1: vec4<f32>,
    @location(9) normal_2: vec4<f32>,
    @location(10) tint: vec4<f32>,
    @location(11) params: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) view_depth: f32,
    @location(3) glow: f32,
    @location(4) world_position: vec3<f32>,
    @location(5) params: vec4<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(in.model_0, in.model_1, in.model_2, in.model_3);
    let world = model * vec4<f32>(in.position, 1.0);
    let normal_matrix = mat3x3<f32>(in.normal_0.xyz, in.normal_1.xyz, in.normal_2.xyz);
    var out: VertexOut;
    out.clip = scene.view_projection * world;
    // The clip w is the view-space depth, which is what fog measures.
    out.view_depth = out.clip.w;
    out.normal = normalize(normal_matrix * in.normal);
    out.color = mix(in.color, in.tint.rgb, in.tint.w);
    // The glow rides a padding float of the normal matrix.
    out.glow = in.normal_0.w;
    out.world_position = world.xyz;
    out.params = in.params;
    return out;
}
"#;

/// The builtin surface: diffuse against the scene light over an ambient
/// floor, glow pulling toward full brightness, then fog.
const MESH_SHADER_LIT_FS: &str = r#"
@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    let facing = max(dot(normal, scene.light.xyz), 0.0);
    // The 1.4 overdrive saturates faces near the sun so a white material can
    // actually render white. It mirrors SUN_STRENGTH in game3d.rs, and the
    // two must move together.
    let shaded = min(scene.light.w + (1.0 - scene.light.w) * facing * 1.4, 1.0);
    // A glowing mesh supplies its own light: glow pulls the shading toward
    // full brightness however the scene is lit, and burns through fog in
    // the same proportion, the way headlights and the sun cut through real
    // haze while the hills behind them vanish.
    let brightness = shaded + (1.0 - shaded) * in.glow;
    var color = in.color * brightness * scene.light_color.rgb;
    let fog_thickness = clamp((in.view_depth - scene.fog_range.x) / max(scene.fog_range.y - scene.fog_range.x, 1e-6), 0.0, 1.0);
    color = mix(color, scene.fog_color.rgb, fog_thickness * scene.fog_color.w * (1.0 - in.glow));
    return vec4<f32>(color, 1.0);
}
"#;

/// The fragment stage wrapped around a program's `shade`: build the
/// surface, let the shader colour it, then fog it exactly like the builtin
/// path - fog is the scene's air, not the shader's business.
const MESH_SHADER_CUSTOM_FS: &str = r#"
@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var surface: NAIL_Surface;
    surface.color = in.color;
    surface.normal = normalize(in.normal);
    surface.world_position = in.world_position;
    surface.toward_eye = normalize(scene.eye.xyz - in.world_position);
    surface.light = scene.light.xyz;
    surface.light_color = scene.light_color.rgb;
    surface.ambient = scene.light.w;
    surface.time = scene.eye.w;
    surface.glow = in.glow;
    surface.params = in.params;
    var color = shade(surface).rgb;
    let fog_thickness = clamp((in.view_depth - scene.fog_range.x) / max(scene.fog_range.y - scene.fog_range.x, 1e-6), 0.0, 1.0);
    color = mix(color, scene.fog_color.rgb, fog_thickness * scene.fog_color.w * (1.0 - in.glow));
    return vec4<f32>(color, 1.0);
}
"#;

/// One program shader as the card will see it: the shared module, the
/// program's WGSL, the wrapper.
pub(crate) fn compose_custom_shader(snippet: &str) -> String {
    return format!("{}\n{}\n{}", MESH_SHADER_COMMON, snippet, MESH_SHADER_CUSTOM_FS);
}

/// Compiles a custom shader exactly the way the card will, so a broken one
/// fails inside `game3d_shader` with the compiler's message instead of at
/// first draw. Runs the same on a machine with no card at all.
pub(crate) fn validate_custom_shader(snippet: &str) -> Result<(), String> {
    let composed = compose_custom_shader(snippet);
    let module = naga::front::wgsl::parse_str(&composed).map_err(|error| format!("game3d_shader: {}", error.emit_to_string(&composed)))?;
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
        .validate(&module)
        .map_err(|error| format!("game3d_shader: {}", error.emit_to_string(&composed)))?;
    return Ok(());
}

const SHAPE2D_SHADER: &str = r#"
struct FlatUniform {
    // target width, target height, coordinate scale, crisp flag
    frame: vec4<f32>,
    // sRGB decode flag, three spares
    encoding: vec4<f32>,
};
@group(0) @binding(0) var<uniform> flat: FlatUniform;
@group(1) @binding(0) var picture: texture_2d<f32>;
@group(1) @binding(1) var picture_sampler: sampler;

struct ShapeIn {
    // kind, origin x, origin y, spare
    @location(0) head: vec4<f32>,
    // the quad's two edge vectors
    @location(1) axes: vec4<f32>,
    // texture window: min xy, max xy
    @location(2) window: vec4<f32>,
    // premultiplied
    @location(3) color: vec4<f32>,
    // signed-distance parameters per kind
    @location(4) params: vec4<f32>,
};

struct ShapeOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) local: vec2<f32>,
    @location(3) params: vec4<f32>,
    @location(4) kind: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32, in: ShapeIn) -> ShapeOut {
    // Six vertices make the quad. A triangle uses the first three as its
    // own corners and collapses the rest to nothing.
    var quad = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0)
    );
    var lone = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 0.0)
    );
    var corner = quad[index];
    if (in.head.x == 3.0) {
        corner = lone[index];
    }
    let position = in.head.yz + corner.x * in.axes.xy + corner.y * in.axes.zw;
    let scaled = position * flat.frame.z;
    let ndc = (scaled / flat.frame.xy) * 2.0 - vec2<f32>(1.0, 1.0);
    var out: ShapeOut;
    out.clip = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    out.uv = mix(in.window.xy, in.window.zw, corner);
    out.color = in.color;
    out.local = (corner - vec2<f32>(0.5, 0.5)) * vec2<f32>(length(in.axes.xy), length(in.axes.zw));
    out.params = in.params;
    out.kind = in.head.x;
    return out;
}

fn signed_box(point: vec2<f32>, half_size: vec2<f32>) -> f32 {
    let q = abs(point) - half_size;
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0);
}

// Distance to opacity: a one-pixel soft edge normally, a hard cut when the
// frame is chunky, matching the CPU rasterizer's crisp mode.
fn edge(distance: f32) -> f32 {
    if (flat.frame.w > 0.5) {
        return select(0.0, 1.0, distance <= 0.0);
    }
    return clamp(0.5 - distance, 0.0, 1.0);
}

fn srgb_channel(channel: f32) -> f32 {
    if (channel <= 0.04045) {
        return channel / 12.92;
    }
    return pow((channel + 0.055) / 1.055, 2.4);
}

@fragment
fn fs_main(in: ShapeOut) -> @location(0) vec4<f32> {
    let sample = textureSample(picture, picture_sampler, in.uv);
    var opacity = 1.0;
    if (in.kind == 1.0) {
        opacity = edge(length(in.local) - in.params.x);
    }
    if (in.kind == 2.0) {
        opacity = edge(abs(signed_box(in.local, in.params.xy)) - in.params.z);
    }
    var out = sample * in.color * opacity;
    if (flat.encoding.x > 0.5) {
        out = vec4<f32>(srgb_channel(out.r), srgb_channel(out.g), srgb_channel(out.b), out.a);
    }
    return out;
}
"#;

const QUAD_SHADER: &str = r#"
struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    // One triangle large enough to cover the whole target.
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOut;
    out.clip = vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(corner.x, 1.0 - corner.y);
    return out;
}

@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var picture_sampler: sampler;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(picture, picture_sampler, in.uv);
}

// For a surface that insists on sRGB encoding: the game's colours are the
// raw numbers the program wrote, so they are decoded here and the surface's
// own encoding puts them back exactly.
fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        return channel / 12.92;
    }
    return pow((channel + 0.055) / 1.055, 2.4);
}

@fragment
fn fs_main_srgb(in: VertexOut) -> @location(0) vec4<f32> {
    let sample = textureSample(picture, picture_sampler, in.uv);
    return vec4<f32>(srgb_to_linear(sample.r), srgb_to_linear(sample.g), srgb_to_linear(sample.b), sample.a);
}
"#;

/// One frame cut into the parts the card draws in order: runs of 2D shapes
/// and whole 3D scenes.
enum FramePart {
    /// Indexes into the frame's shapes: a run with no scene3d inside it.
    Layer(std::ops::Range<usize>),
    Scene(game3d::SceneData),
}

/// A texture with the bind group that draws it: the glyph atlas, each
/// uploaded sprite, and the offscreen picture all take this shape.
struct BoundTexture {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

/// What a run of 2D instances samples from.
#[derive(Clone, Copy, PartialEq)]
enum BindSource {
    /// The shared atlas: solid colours and text.
    Atlas,
    /// One loaded sprite's own texture.
    Sprite(i64),
}

/// One instanced draw call: `count` shapes starting `byte_offset` into the
/// instance buffer. The offset is bound rather than drawn from, because
/// WebGL2 has no first-instance.
struct Batch2D {
    bind: BindSource,
    byte_offset: u64,
    count: u32,
}

/// One glyph's home in the atlas and the metrics that place it, cached the
/// first time a character is drawn at a size.
#[derive(Clone, Copy)]
struct GlyphSlot {
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    width: f32,
    height: f32,
    left_offset: f32,
    bottom_offset: f32,
    advance: f32,
}

/// Where the picture goes: a desktop window or a browser canvas.
pub(crate) enum RenderTarget {
    #[cfg(not(target_arch = "wasm32"))]
    Window(std::sync::Arc<winit::window::Window>),
    #[cfg(target_arch = "wasm32")]
    Canvas(web_sys::HtmlCanvasElement),
}

pub(crate) struct GpuRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    adapter_name: String,
    backend_name: String,

    mesh_pipeline: wgpu::RenderPipeline,
    /// The mesh pipeline's layout, kept so a pipeline for a shader loaded
    /// with `game3d_shader` can be built the first time a draw uses it.
    mesh_pipeline_layout: wgpu::PipelineLayout,
    custom_pipelines: HashMap<i64, wgpu::RenderPipeline>,
    /// 2D shapes into the offscreen picture, and 2D text straight onto the
    /// window at full resolution when the picture is chunky.
    shape_pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,
    present_pipeline: wgpu::RenderPipeline,
    quad_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,

    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_stride: u32,
    uniform_capacity: usize,

    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,

    /// The two fixed 2D uniform blocks: the offscreen picture's, then the
    /// full-resolution window's. Written once, the sizes never change.
    flat_uniform_bind_group: wgpu::BindGroup,
    flat_uniform_stride: u32,
    shape_instance_buffer: wgpu::Buffer,
    shape_instance_capacity: usize,

    /// The low-resolution picture every part draws into, its multisampled
    /// twin when antialiasing is on, and the depth buffer scenes share.
    offscreen_view: wgpu::TextureView,
    offscreen_bind_group: wgpu::BindGroup,
    multisampled_view: Option<wgpu::TextureView>,
    depth_view: wgpu::TextureView,
    sample_count: u32,

    /// The atlas: a white pixel for solid shapes, and every glyph drawn so
    /// far. A simple shelf packer fills it left to right, top to bottom.
    atlas: BoundTexture,
    atlas_pen_x: u32,
    atlas_pen_y: u32,
    atlas_row_height: u32,
    glyphs: HashMap<(char, u32), GlyphSlot>,
    white_uv: [f32; 2],

    /// Vertex buffers per mesh handle and textures per sprite handle,
    /// uploaded on first sight.
    mesh_buffers: HashMap<i64, (wgpu::Buffer, u32)>,
    sprite_textures: HashMap<i64, BoundTexture>,

    logical_width: u32,
    logical_height: u32,
    picture_width: u32,
    picture_height: u32,
}

impl GpuRenderer {
    /// Reaches the graphics card and builds everything a frame needs. Any
    /// failure is a `Device` failure: the machine has no card worth using
    /// and the CPU path takes over.
    pub(crate) async fn create(target: RenderTarget, logical_width: u32, logical_height: u32, pixel_size: u32) -> Result<GpuRenderer, GpuFrameError> {
        let device_problem = |what: &str, why: String| GpuFrameError::Device(format!("{}: {}", what, why));

        // The browser path pins itself to WebGL2, the one backend every
        // browser has. The desktop takes whatever is native and falls back
        // to GL.
        #[cfg(target_arch = "wasm32")]
        let backends = wgpu::Backends::GL;
        #[cfg(not(target_arch = "wasm32"))]
        let backends = wgpu::Backends::PRIMARY | wgpu::Backends::GL;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends, ..Default::default() });
        let surface = match target {
            #[cfg(not(target_arch = "wasm32"))]
            RenderTarget::Window(window) => instance.create_surface(window).map_err(|e| device_problem("could not make a drawing surface", e.to_string()))?,
            #[cfg(target_arch = "wasm32")]
            RenderTarget::Canvas(canvas) => instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)).map_err(|e| device_problem("could not make a drawing surface", e.to_string()))?,
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| device_problem("no graphics adapter answered", "the machine offered none".to_string()))?;
        let info = adapter.get_info();
        let adapter_name = info.name.clone();
        let backend_name = info.backend.to_string();

        // WebGL2's limits are the floor everywhere, so one set of limits
        // serves both worlds without asking for anything a browser lacks.
        let limits = wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("nail game"),
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| device_problem("the adapter would not give a device", e.to_string()))?;

        let capabilities = surface.get_capabilities(&adapter);
        if capabilities.formats.is_empty() {
            return Err(device_problem("no usable surface colour format", "the surface offers none".to_string()));
        }
        // The CPU path writes colours exactly as the program named them, no
        // gamma arithmetic anywhere. A non-sRGB surface keeps the card
        // honest to the same numbers, and a surface that only offers sRGB
        // gets the decode done in the shaders that touch it instead.
        let surface_format = capabilities.formats.iter().copied().find(|format| !format.is_srgb()).unwrap_or(capabilities.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: logical_width.max(1),
            height: logical_height.max(1),
            // Pacing belongs to the game loop's own frame budget, the same
            // way the CPU path paces, so the surface must not also wait for
            // the screen.
            present_mode: wgpu::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let picture_width = (logical_width / pixel_size).max(1);
        let picture_height = (logical_height / pixel_size).max(1);

        // Full-resolution frames get antialiased edges when the card offers
        // them. Chunky-pixel frames want hard edges, exactly like the CPU
        // rasterizer's crisp mode.
        let format_flags = adapter.get_texture_format_features(wgpu::TextureFormat::Rgba8Unorm).flags;
        let sample_count = if pixel_size == 1 && format_flags.sample_count_supported(4) { 4 } else { 1 };

        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("nail mesh shader"), source: wgpu::ShaderSource::Wgsl(Cow::Owned(format!("{}\n{}", MESH_SHADER_COMMON, MESH_SHADER_LIT_FS))) });
        let shape_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("nail shape shader"), source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHAPE2D_SHADER)) });
        let quad_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("nail quad shader"), source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(QUAD_SHADER)) });

        let uniform_stride = device.limits().min_uniform_buffer_offset_alignment.max((UNIFORM_FLOATS * 4) as u32);
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nail scene uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new((UNIFORM_FLOATS * 4) as u64),
                },
                count: None,
            }],
        });
        let flat_uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nail flat uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(32),
                },
                count: None,
            }],
        });
        let quad_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nail picture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let mesh_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("nail mesh"), bind_group_layouts: &[&uniform_layout], push_constant_ranges: &[] });
        let shape_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("nail shape"), bind_group_layouts: &[&flat_uniform_layout, &quad_layout], push_constant_ranges: &[] });
        let quad_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("nail quad"), bind_group_layouts: &[&quad_layout], push_constant_ranges: &[] });

        let shape_attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4];
        let premultiplied = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, operation: wgpu::BlendOperation::Add },
        };

        let mesh_pipeline = build_mesh_pipeline(&device, &mesh_pipeline_layout, sample_count, &mesh_shader, "nail mesh");

        let shape_pipeline_for = |label: &str, format: wgpu::TextureFormat, samples: u32, depth: bool| {
            return device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&shape_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shape_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout { array_stride: (SHAPE2D_FLOATS * 4) as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &shape_attributes }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shape_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend: Some(premultiplied), write_mask: wgpu::ColorWrites::ALL })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: if depth {
                    Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: false,
                        depth_compare: wgpu::CompareFunction::Always,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    })
                } else {
                    None
                },
                multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
                multiview: None,
                cache: None,
            });
        };
        // Shapes share the offscreen picture's sample count and depth
        // attachment. Text at full resolution draws onto the window itself.
        let shape_pipeline = shape_pipeline_for("nail shapes", wgpu::TextureFormat::Rgba8Unorm, sample_count, true);
        let text_pipeline = shape_pipeline_for("nail text", surface_format, 1, false);

        let present_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nail present"),
            layout: Some(&quad_pipeline_layout),
            vertex: wgpu::VertexState { module: &quad_shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState {
                module: &quad_shader,
                entry_point: Some(if surface_format.is_srgb() { "fs_main_srgb" } else { "fs_main" }),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: surface_format, blend: Some(premultiplied), write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform_capacity = 16;
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nail scene uniforms"),
            size: uniform_stride as u64 * uniform_capacity as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nail scene uniforms"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &uniform_buffer, offset: 0, size: wgpu::BufferSize::new((UNIFORM_FLOATS * 4) as u64) }),
            }],
        });

        // The two 2D uniform blocks never change: the picture's size and
        // scale, then the window's. Written here, bound by offset later.
        let flat_uniform_stride = device.limits().min_uniform_buffer_offset_alignment.max(32);
        let flat_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nail flat uniforms"),
            size: flat_uniform_stride as u64 * 2,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let crisp = if pixel_size > 1 { 1.0f32 } else { 0.0 };
        let srgb_flag = if surface_format.is_srgb() { 1.0f32 } else { 0.0 };
        let picture_block: [f32; 8] = [picture_width as f32, picture_height as f32, 1.0 / pixel_size as f32, crisp, 0.0, 0.0, 0.0, 0.0];
        let window_block: [f32; 8] = [logical_width as f32, logical_height as f32, 1.0, 0.0, srgb_flag, 0.0, 0.0, 0.0];
        queue.write_buffer(&flat_uniform_buffer, 0, bytemuck::cast_slice(&picture_block));
        queue.write_buffer(&flat_uniform_buffer, flat_uniform_stride as u64, bytemuck::cast_slice(&window_block));
        let flat_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nail flat uniforms"),
            layout: &flat_uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &flat_uniform_buffer, offset: 0, size: wgpu::BufferSize::new(32) }),
            }],
        });

        let instance_capacity = 1024;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nail instances"),
            size: (instance_capacity * INSTANCE_FLOATS * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shape_instance_capacity = 4096;
        let shape_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nail 2d instances"),
            size: (shape_instance_capacity * SHAPE2D_FLOATS * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nail nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let make_texture = |label: &str, width: u32, height: u32, samples: u32, format: wgpu::TextureFormat, usage: wgpu::TextureUsages| {
            return device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            });
        };
        let offscreen = make_texture("nail offscreen", picture_width, picture_height, 1, wgpu::TextureFormat::Rgba8Unorm, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
        let offscreen_view = offscreen.create_view(&wgpu::TextureViewDescriptor::default());
        let offscreen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nail offscreen"),
            layout: &quad_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&offscreen_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let multisampled_view = if sample_count > 1 {
            Some(make_texture("nail offscreen msaa", picture_width, picture_height, sample_count, wgpu::TextureFormat::Rgba8Unorm, wgpu::TextureUsages::RENDER_ATTACHMENT).create_view(&wgpu::TextureViewDescriptor::default()))
        } else {
            None
        };
        let depth_view = make_texture("nail depth", picture_width, picture_height, sample_count, wgpu::TextureFormat::Depth32Float, wgpu::TextureUsages::RENDER_ATTACHMENT).create_view(&wgpu::TextureViewDescriptor::default());

        // The atlas starts as a white square in its top-left corner, which
        // is all a solid-colour shape ever samples.
        let atlas_texture = make_texture("nail atlas", ATLAS_SIZE, ATLAS_SIZE, 1, wgpu::TextureFormat::Rgba8Unorm, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nail atlas"),
            layout: &quad_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &atlas_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &[0xffu8; 4 * 4 * 4],
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * 4), rows_per_image: Some(4) },
            wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
        );
        let atlas = BoundTexture { texture: atlas_texture, bind_group: atlas_bind_group, width: ATLAS_SIZE, height: ATLAS_SIZE };
        let white_uv = [2.0 / ATLAS_SIZE as f32, 2.0 / ATLAS_SIZE as f32];

        return Ok(GpuRenderer {
            surface,
            device,
            queue,
            surface_config,
            adapter_name,
            backend_name,
            mesh_pipeline,
            mesh_pipeline_layout,
            custom_pipelines: HashMap::new(),
            shape_pipeline,
            text_pipeline,
            present_pipeline,
            quad_layout,
            sampler,
            uniform_buffer,
            uniform_bind_group,
            uniform_stride,
            uniform_capacity,
            instance_buffer,
            instance_capacity,
            flat_uniform_bind_group,
            flat_uniform_stride,
            shape_instance_buffer,
            shape_instance_capacity,
            offscreen_view,
            offscreen_bind_group,
            multisampled_view,
            depth_view,
            sample_count,
            atlas,
            atlas_pen_x: 6,
            atlas_pen_y: 0,
            atlas_row_height: 6,
            glyphs: HashMap::new(),
            white_uv,
            mesh_buffers: HashMap::new(),
            sprite_textures: HashMap::new(),
            logical_width,
            logical_height,
            picture_width,
            picture_height,
        });
    }

    /// One line naming what the game is actually drawing on, for the
    /// stderr note when a window opens.
    pub(crate) fn describe(&self) -> String {
        return format!("{} ({})", self.adapter_name, self.backend_name);
    }

    /// Draws one whole frame: background, 2D shapes and 3D scenes in shape
    /// order onto the offscreen picture, then the picture onto the window
    /// with full-resolution text over it.
    pub(crate) fn render_frame(&mut self, frame: &GAME_Frame, pixel_size: u32) -> Result<(), GpuFrameError> {
        let background = super::game::parse_color(&frame.background).map_err(GpuFrameError::Program)?;

        // Cut the frame into parts and take each scene's data out of the
        // store, which is also what keeps the store from growing.
        let mut parts: Vec<FramePart> = Vec::new();
        let mut run_start = 0usize;
        for (index, shape) in frame.shapes.iter().enumerate() {
            if shape.kind == "scene3d" {
                if run_start < index {
                    parts.push(FramePart::Layer(run_start..index));
                }
                let Some(data) = game3d::take_scene(shape.sprite) else {
                    return Err(GpuFrameError::Program("game_run: a scene3d shape refers to a scene that was already drawn - game3d_scene makes a fresh one each frame".to_string()));
                };
                parts.push(FramePart::Scene(data));
                run_start = index + 1;
            }
        }
        if run_start < frame.shapes.len() {
            parts.push(FramePart::Layer(run_start..frame.shapes.len()));
        }

        // Build every scene's uniforms and instances before touching the
        // card, so program mistakes surface before anything is half-drawn.
        let mut uniforms: Vec<f32> = Vec::new();
        let mut instances: Vec<f32> = Vec::new();
        // Per scene: the batches to draw, as (mesh handle, shader handle,
        // byte offset into the instance buffer, instance count).
        let mut scene_batches: Vec<Vec<(i64, i64, u64, u32)>> = Vec::new();
        let uniform_stride_floats = self.uniform_stride as usize / 4;
        for part in &parts {
            let FramePart::Scene(data) = part else { continue };
            let scene_index = scene_batches.len();
            uniforms.resize((scene_index + 1) * uniform_stride_floats, 0.0);
            let block = &mut uniforms[scene_index * uniform_stride_floats..];
            fill_scene_uniform(block, data).map_err(GpuFrameError::Program)?;

            // Group draws by mesh and shader so each pair is one instanced
            // draw call. Handle 0 is the builtin shading.
            let mut batches: Vec<(i64, i64, u64, u32)> = Vec::new();
            let mut by_batch: HashMap<(i64, i64), Vec<&game3d::GAME3D_Draw>> = HashMap::new();
            let mut batch_order: Vec<(i64, i64)> = Vec::new();
            for draw in &data.draws {
                let key = (draw.mesh, draw.shader);
                if !by_batch.contains_key(&key) {
                    batch_order.push(key);
                }
                by_batch.entry(key).or_default().push(draw);
            }
            for (mesh, shader) in batch_order {
                self.ensure_mesh_uploaded(mesh).map_err(GpuFrameError::Program)?;
                self.ensure_shader_pipeline(shader).map_err(GpuFrameError::Program)?;
                let draws = &by_batch[&(mesh, shader)];
                let offset = (instances.len() * 4) as u64;
                for draw in draws {
                    push_instance(&mut instances, draw).map_err(GpuFrameError::Program)?;
                }
                batches.push((mesh, shader, offset, draws.len() as u32));
            }
            scene_batches.push(batches);
        }

        // Build every 2D shape into instances, layer by layer. Text keeps
        // full resolution when the picture is chunky, so its instances land
        // in the separate window-resolution list drawn after the stretch.
        let mut shape_floats: Vec<f32> = Vec::new();
        let mut layer_batches: Vec<Vec<Batch2D>> = Vec::new();
        let mut window_text: Vec<f32> = Vec::new();
        let mut window_batches: Vec<Batch2D> = Vec::new();
        let text_on_window = pixel_size > 1;
        for part in &parts {
            let FramePart::Layer(range) = part else { continue };
            let mut batches: Vec<Batch2D> = Vec::new();
            for shape in &frame.shapes[range.clone()] {
                // A scrolling game hands over its whole world every frame,
                // and most of it is off screen - same skip as the CPU path.
                if let Some((min_x, min_y, max_x, max_y)) = super::game::shape_bounds(shape) {
                    if max_x < 0.0 || min_x > self.logical_width as f64 || max_y < 0.0 || min_y > self.logical_height as f64 {
                        continue;
                    }
                }
                if shape.kind == "text" && text_on_window {
                    self.push_text(&mut window_text, &mut window_batches, shape).map_err(GpuFrameError::Program)?;
                } else if shape.kind == "text" {
                    self.push_text(&mut shape_floats, &mut batches, shape).map_err(GpuFrameError::Program)?;
                } else {
                    self.push_shape(&mut shape_floats, &mut batches, shape).map_err(GpuFrameError::Program)?;
                }
            }
            layer_batches.push(batches);
        }
        // The window-resolution text sits in the same buffer after the
        // picture-resolution shapes, so its batch offsets shift by that.
        let window_base = (shape_floats.len() * 4) as u64;
        for batch in &mut window_batches {
            batch.byte_offset += window_base;
        }
        shape_floats.extend_from_slice(&window_text);

        self.grow_buffers_if_needed(uniforms.len() / uniform_stride_floats.max(1), instances.len() / INSTANCE_FLOATS, shape_floats.len() / SHAPE2D_FLOATS);
        if !uniforms.is_empty() {
            self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));
        }
        if !instances.is_empty() {
            self.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        }
        if !shape_floats.is_empty() {
            self.queue.write_buffer(&self.shape_instance_buffer, 0, bytemuck::cast_slice(&shape_floats));
        }

        // The window's real size can drift from the logical size on a
        // high-DPI screen. The blit stretches, so only the surface needs
        // reconfiguring.
        let surface_texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Lost) | Err(wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_config);
                self.surface.get_current_texture().map_err(|e| GpuFrameError::Device(format!("the drawing surface will not come back: {}", e)))?
            }
            Err(wgpu::SurfaceError::Timeout) => {
                // One missed frame is a skipped frame, not a dead game.
                return Ok(());
            }
            Err(other) => {
                return Err(GpuFrameError::Device(format!("the drawing surface failed: {}", other)));
            }
        };
        let surface_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let clear_color = wgpu::Color {
            r: background.red() as f64,
            g: background.green() as f64,
            b: background.blue() as f64,
            a: 1.0,
        };

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("nail frame") });
        // Every part draws onto the offscreen picture in its own pass: the
        // first clears to the background, later ones load what is there,
        // and each scene clears the depth buffer for itself so scenes never
        // fight each other's leftovers.
        let mut first_pass = true;
        let mut layer_index = 0usize;
        let mut scene_index = 0usize;
        for part in &parts {
            let load = if first_pass { wgpu::LoadOp::Clear(clear_color) } else { wgpu::LoadOp::Load };
            first_pass = false;
            let (view, resolve_target) = match self.multisampled_view.as_ref() {
                Some(multisampled) => (multisampled, Some(&self.offscreen_view)),
                None => (&self.offscreen_view, None),
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nail part"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view, resolve_target, ops: wgpu::Operations { load, store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            match part {
                FramePart::Layer(_) => {
                    pass.set_pipeline(&self.shape_pipeline);
                    pass.set_bind_group(0, &self.flat_uniform_bind_group, &[0]);
                    for batch in &layer_batches[layer_index] {
                        let bind = match batch.bind {
                            BindSource::Atlas => &self.atlas.bind_group,
                            BindSource::Sprite(handle) => &self.sprite_textures[&handle].bind_group,
                        };
                        pass.set_bind_group(1, bind, &[]);
                        pass.set_vertex_buffer(0, self.shape_instance_buffer.slice(batch.byte_offset..));
                        pass.draw(0..6, 0..batch.count);
                    }
                    layer_index += 1;
                }
                FramePart::Scene(_) => {
                    let offset = scene_index as u32 * self.uniform_stride;
                    pass.set_bind_group(0, &self.uniform_bind_group, &[offset]);
                    for (mesh, shader, byte_offset, count) in &scene_batches[scene_index] {
                        match *shader == 0 {
                            true => pass.set_pipeline(&self.mesh_pipeline),
                            false => pass.set_pipeline(&self.custom_pipelines[shader]),
                        }
                        let (buffer, vertex_count) = &self.mesh_buffers[mesh];
                        pass.set_vertex_buffer(0, buffer.slice(..));
                        pass.set_vertex_buffer(1, self.instance_buffer.slice(*byte_offset..));
                        pass.draw(0..*vertex_count, 0..*count);
                    }
                    scene_index += 1;
                }
            }
        }
        if first_pass {
            // A frame with no shapes at all still clears to its background.
            let (view, resolve_target) = match self.multisampled_view.as_ref() {
                Some(multisampled) => (multisampled, Some(&self.offscreen_view)),
                None => (&self.offscreen_view, None),
            };
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nail empty frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view, resolve_target, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear_color), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
        }

        // The picture goes onto the window nearest-neighbour, chunky
        // pixels intact, and full-resolution text goes over it.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nail present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.present_pipeline);
            pass.set_bind_group(0, &self.offscreen_bind_group, &[]);
            pass.draw(0..3, 0..1);
            if !window_batches.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.flat_uniform_bind_group, &[self.flat_uniform_stride]);
                for batch in &window_batches {
                    let bind = match batch.bind {
                        BindSource::Atlas => &self.atlas.bind_group,
                        BindSource::Sprite(handle) => &self.sprite_textures[&handle].bind_group,
                    };
                    pass.set_bind_group(1, bind, &[]);
                    pass.set_vertex_buffer(0, self.shape_instance_buffer.slice(batch.byte_offset..));
                    pass.draw(0..6, 0..batch.count);
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
        return Ok(());
    }

    /// The window changed physical size (a high-DPI move between screens),
    /// so the surface follows it. The picture keeps its own resolution.
    pub(crate) fn resize_surface(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if width != self.surface_config.width || height != self.surface_config.height {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    /// Appends one non-text shape's instance, extending the current batch
    /// or opening a new one when the texture changes.
    fn push_shape(&mut self, floats: &mut Vec<f32>, batches: &mut Vec<Batch2D>, shape: &GAME_Shape) -> Result<(), String> {
        let color = premultiplied(&shape.color)?;
        let white = [self.white_uv[0], self.white_uv[1], self.white_uv[0], self.white_uv[1]];
        match shape.kind.as_str() {
            "rect" => {
                if shape.width <= 0.0 || shape.height <= 0.0 {
                    return Ok(());
                }
                extend_batch(batches, BindSource::Atlas, floats.len());
                push_quad(floats, KIND_FILL, [shape.x_coordinate, shape.y_coordinate], [shape.width, 0.0], [0.0, shape.height], white, color, [0.0; 4]);
            }
            "rect_outline" => {
                if shape.width <= 0.0 || shape.height <= 0.0 || shape.thickness <= 0.0 {
                    return Ok(());
                }
                let spread = shape.thickness / 2.0;
                extend_batch(batches, BindSource::Atlas, floats.len());
                push_quad(
                    floats,
                    KIND_OUTLINE,
                    [shape.x_coordinate - spread, shape.y_coordinate - spread],
                    [shape.width + shape.thickness, 0.0],
                    [0.0, shape.height + shape.thickness],
                    white,
                    color,
                    [(shape.width / 2.0) as f32, (shape.height / 2.0) as f32, spread as f32, 0.0],
                );
            }
            "circle" => {
                if shape.radius <= 0.0 {
                    return Ok(());
                }
                extend_batch(batches, BindSource::Atlas, floats.len());
                push_quad(
                    floats,
                    KIND_CIRCLE,
                    [shape.x_coordinate - shape.radius, shape.y_coordinate - shape.radius],
                    [shape.radius * 2.0, 0.0],
                    [0.0, shape.radius * 2.0],
                    white,
                    color,
                    [shape.radius as f32, 0.0, 0.0, 0.0],
                );
            }
            "line" => {
                let Some((origin, along, across)) = line_quad([shape.x_coordinate, shape.y_coordinate], [shape.end_x, shape.end_y], shape.thickness) else {
                    return Ok(());
                };
                extend_batch(batches, BindSource::Atlas, floats.len());
                push_quad(floats, KIND_FILL, origin, along, across, white, color, [0.0; 4]);
            }
            "triangle" => {
                extend_batch(batches, BindSource::Atlas, floats.len());
                push_quad(
                    floats,
                    KIND_TRIANGLE,
                    [shape.x_coordinate, shape.y_coordinate],
                    [shape.end_x - shape.x_coordinate, shape.end_y - shape.y_coordinate],
                    [shape.third_x - shape.x_coordinate, shape.third_y - shape.y_coordinate],
                    white,
                    color,
                    [0.0; 4],
                );
            }
            "sprite" | "sprite_scaled" => {
                self.ensure_sprite_uploaded(shape.sprite)?;
                let sprite = &self.sprite_textures[&shape.sprite];
                let width = if shape.kind == "sprite_scaled" { shape.width } else { sprite.width as f64 };
                let height = if shape.kind == "sprite_scaled" { shape.height } else { sprite.height as f64 };
                extend_batch(batches, BindSource::Sprite(shape.sprite), floats.len());
                push_quad(floats, KIND_FILL, [shape.x_coordinate, shape.y_coordinate], [width, 0.0], [0.0, height], [0.0, 0.0, 1.0, 1.0], [1.0, 1.0, 1.0, 1.0], [0.0; 4]);
            }
            other => {
                return Err(format!("game_run: `{}` is not a shape kind this understands", other));
            }
        }
        return Ok(());
    }

    /// Appends one text shape as a run of glyph quads, laid out with the
    /// same pen arithmetic the CPU rasterizer uses.
    fn push_text(&mut self, floats: &mut Vec<f32>, batches: &mut Vec<Batch2D>, shape: &GAME_Shape) -> Result<(), String> {
        let color = premultiplied_opaque(&shape.color)?;
        let size = shape.size as f32;
        if size <= 0.0 {
            return Ok(());
        }
        // The shape's y is the top of the text, glyphs hang from the
        // baseline below it. The size itself is a workable ascent.
        let baseline = shape.y_coordinate + shape.size;
        let mut pen = shape.x_coordinate;
        for character in shape.text.chars() {
            let slot = self.ensure_glyph(character, size)?;
            if slot.width > 0.0 && slot.height > 0.0 {
                let left = pen + slot.left_offset as f64;
                let top = baseline - slot.height as f64 - slot.bottom_offset as f64;
                extend_batch(batches, BindSource::Atlas, floats.len());
                push_quad(
                    floats,
                    KIND_FILL,
                    [left, top],
                    [slot.width as f64, 0.0],
                    [0.0, slot.height as f64],
                    [slot.uv_min[0], slot.uv_min[1], slot.uv_max[0], slot.uv_max[1]],
                    color,
                    [0.0; 4],
                );
            }
            pen += slot.advance as f64;
        }
        return Ok(());
    }

    /// The glyph's slot in the atlas, rasterized and uploaded the first
    /// time this character appears at this size.
    fn ensure_glyph(&mut self, character: char, size: f32) -> Result<GlyphSlot, String> {
        let key = (character, size.to_bits());
        if let Some(slot) = self.glyphs.get(&key) {
            return Ok(*slot);
        }
        let font = super::game::game_font()?;
        let (metrics, coverage) = font.rasterize(character, size);
        let width = metrics.width as u32;
        let height = metrics.height as u32;
        let mut slot = GlyphSlot {
            uv_min: self.white_uv,
            uv_max: self.white_uv,
            width: 0.0,
            height: 0.0,
            left_offset: metrics.xmin as f32,
            bottom_offset: metrics.ymin as f32,
            advance: metrics.advance_width,
        };
        if width > 0 && height > 0 {
            // Shelf packing with a pixel of breathing room. A game drawing
            // text at endlessly new sizes can genuinely fill this, and the
            // honest answer then is the CPU path, not a corrupted atlas.
            if self.atlas_pen_x + width + 1 > self.atlas.width {
                self.atlas_pen_x = 0;
                self.atlas_pen_y += self.atlas_row_height + 1;
                self.atlas_row_height = 0;
            }
            if self.atlas_pen_y + height + 1 > self.atlas.height {
                return Err("the glyph atlas filled up - this game draws text at too many different sizes for the card".to_string());
            }
            let pen_x = self.atlas_pen_x;
            let pen_y = self.atlas_pen_y;
            self.atlas_pen_x += width + 1;
            self.atlas_row_height = self.atlas_row_height.max(height);

            // Coverage becomes premultiplied white, so a glyph and a solid
            // shape are the same arithmetic in the shader: sample times
            // colour.
            let mut pixels = Vec::with_capacity(coverage.len() * 4);
            for value in &coverage {
                pixels.extend_from_slice(&[*value, *value, *value, *value]);
            }
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &self.atlas.texture, mip_level: 0, origin: wgpu::Origin3d { x: pen_x, y: pen_y, z: 0 }, aspect: wgpu::TextureAspect::All },
                &pixels,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
                wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            );
            let atlas_width = self.atlas.width as f32;
            let atlas_height = self.atlas.height as f32;
            slot.uv_min = [pen_x as f32 / atlas_width, pen_y as f32 / atlas_height];
            slot.uv_max = [(pen_x + width) as f32 / atlas_width, (pen_y + height) as f32 / atlas_height];
            slot.width = width as f32;
            slot.height = height as f32;
        }
        self.glyphs.insert(key, slot);
        return Ok(slot);
    }

    fn ensure_sprite_uploaded(&mut self, handle: i64) -> Result<(), String> {
        if self.sprite_textures.contains_key(&handle) {
            return Ok(());
        }
        let Some((pixels, width, height)) = super::game::sprite_pixels(handle) else {
            return Err(format!("game_run: shape refers to sprite {} but no sprite with that number was loaded", handle));
        };
        let texture = self.make_bound_texture("nail sprite", width.max(1), height.max(1));
        upload_rgba(&self.queue, &texture.texture, &pixels, width, height);
        self.sprite_textures.insert(handle, texture);
        return Ok(());
    }

    fn ensure_mesh_uploaded(&mut self, handle: i64) -> Result<(), String> {
        if self.mesh_buffers.contains_key(&handle) {
            return Ok(());
        }
        let floats = game3d::mesh_vertex_floats(handle).map_err(|why| {
            if why.contains("poisoned") {
                return why;
            }
            // The same words the CPU fallback uses for the same mistake.
            return format!("game3d_scene: a draw refers to mesh {} but no mesh with that number was loaded", handle);
        })?;
        use wgpu::util::DeviceExt;
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nail mesh vertices"),
            contents: bytemuck::cast_slice(&floats),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.mesh_buffers.insert(handle, (buffer, (floats.len() / 9) as u32));
        return Ok(());
    }

    /// Builds and caches the pipeline for one loaded shader the first time
    /// a draw uses it. The WGSL was already validated when the program
    /// loaded it, so a failure here is a device problem, not a typo.
    fn ensure_shader_pipeline(&mut self, handle: i64) -> Result<(), String> {
        if handle == 0 || self.custom_pipelines.contains_key(&handle) {
            return Ok(());
        }
        let composed = compose_custom_shader(&game3d::shader_source(handle)?);
        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("nail custom shader"), source: wgpu::ShaderSource::Wgsl(Cow::Owned(composed)) });
        let pipeline = build_mesh_pipeline(&self.device, &self.mesh_pipeline_layout, self.sample_count, &module, "nail custom mesh");
        self.custom_pipelines.insert(handle, pipeline);
        return Ok(());
    }

    fn grow_buffers_if_needed(&mut self, scenes: usize, instance_count: usize, shape_count: usize) {
        if scenes > self.uniform_capacity {
            self.uniform_capacity = scenes.next_power_of_two();
            self.uniform_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("nail scene uniforms"),
                size: self.uniform_stride as u64 * self.uniform_capacity as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.uniform_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("nail scene uniforms"),
                layout: &self.mesh_pipeline.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &self.uniform_buffer, offset: 0, size: wgpu::BufferSize::new((UNIFORM_FLOATS * 4) as u64) }),
                }],
            });
        }
        if instance_count > self.instance_capacity {
            self.instance_capacity = instance_count.next_power_of_two();
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("nail instances"),
                size: (self.instance_capacity * INSTANCE_FLOATS * 4) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if shape_count > self.shape_instance_capacity {
            self.shape_instance_capacity = shape_count.next_power_of_two();
            self.shape_instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("nail 2d instances"),
                size: (self.shape_instance_capacity * SHAPE2D_FLOATS * 4) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
    }

    fn make_bound_texture(&self, label: &str, width: u32, height: u32) -> BoundTexture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.quad_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        return BoundTexture { texture, bind_group, width, height };
    }
}

/// Extends the last batch when it samples the same texture, opens a new one
/// when it does not - which is all that painter's order costs on a card.
fn extend_batch(batches: &mut Vec<Batch2D>, bind: BindSource, float_offset: usize) {
    if let Some(last) = batches.last_mut() {
        if last.bind == bind {
            last.count += 1;
            return;
        }
    }
    batches.push(Batch2D { bind, byte_offset: (float_offset * 4) as u64, count: 1 });
}

/// Appends the five vec4s of one 2D instance.
fn push_quad(floats: &mut Vec<f32>, kind: f32, origin: [f64; 2], axis_x: [f64; 2], axis_y: [f64; 2], window: [f32; 4], color: [f32; 4], params: [f32; 4]) {
    floats.extend_from_slice(&[kind, origin[0] as f32, origin[1] as f32, 0.0]);
    floats.extend_from_slice(&[axis_x[0] as f32, axis_x[1] as f32, axis_y[0] as f32, axis_y[1] as f32]);
    floats.extend_from_slice(&window);
    floats.extend_from_slice(&color);
    floats.extend_from_slice(&params);
}

/// The rectangle a stroked line covers: butt caps, `thickness` wide. None
/// when the line has no length or no width to draw.
fn line_quad(from: [f64; 2], to: [f64; 2], thickness: f64) -> Option<([f64; 2], [f64; 2], [f64; 2])> {
    let along = [to[0] - from[0], to[1] - from[1]];
    let length = (along[0] * along[0] + along[1] * along[1]).sqrt();
    if length < 1e-9 || thickness <= 0.0 {
        return None;
    }
    let across = [-along[1] / length * thickness / 2.0, along[0] / length * thickness / 2.0];
    let origin = [from[0] - across[0], from[1] - across[1]];
    return Some((origin, along, [across[0] * 2.0, across[1] * 2.0]));
}

/// A shape colour as premultiplied rgba, the form the blend state expects.
fn premultiplied(color: &str) -> Result<[f32; 4], String> {
    let parsed = super::game::parse_color(color)?;
    let alpha = parsed.alpha();
    return Ok([parsed.red() * alpha, parsed.green() * alpha, parsed.blue() * alpha, alpha]);
}

/// Text colour: the CPU rasterizer reads only the channels and lets glyph
/// coverage be the alpha, so the GPU does the same.
fn premultiplied_opaque(color: &str) -> Result<[f32; 4], String> {
    let parsed = super::game::parse_color(color)?;
    return Ok([parsed.red(), parsed.green(), parsed.blue(), 1.0]);
}

fn upload_rgba(queue: &wgpu::Queue, texture: &wgpu::Texture, pixels: &[u8], width: u32, height: u32) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        pixels,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
}

/// One mesh pipeline: the shared vertex stage and whichever fragment shader
/// this module carries, the builtin one or a program's own loaded through
/// `game3d_shader`.
fn build_mesh_pipeline(device: &wgpu::Device, layout: &wgpu::PipelineLayout, sample_count: u32, module: &wgpu::ShaderModule, label: &str) -> wgpu::RenderPipeline {
    let vertex_attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];
    let instance_attributes = wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4, 10 => Float32x4, 11 => Float32x4];
    return device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[
                wgpu::VertexBufferLayout { array_stride: 9 * 4, step_mode: wgpu::VertexStepMode::Vertex, attributes: &vertex_attributes },
                wgpu::VertexBufferLayout { array_stride: (INSTANCE_FLOATS * 4) as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &instance_attributes },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState { count: sample_count, ..Default::default() },
        multiview: None,
        cache: None,
    });
}

/// Seconds since the process first drew - the clock a custom shader reads
/// as `surface.time`.
fn clock_seconds() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        return STARTED.get_or_init(std::time::Instant::now).elapsed().as_secs_f64();
    }
    #[cfg(target_arch = "wasm32")]
    {
        return web_sys::window().and_then(|window| window.performance()).map(|performance| performance.now() / 1000.0).unwrap_or(0.0);
    }
}

/// Writes one scene's uniform block: the view-projection matrix built from
/// the camera exactly the way the CPU projector projects, then the light,
/// its colour, and the fog.
fn fill_scene_uniform(block: &mut [f32], data: &game3d::SceneData) -> Result<(), String> {
    let camera = &data.camera;
    let environment = &data.environment;

    let eye = [camera.position_x, camera.position_y, camera.position_z];
    let forward = normalized([camera.target_x - eye[0], camera.target_y - eye[1], camera.target_z - eye[2]]);
    let right = normalized(cross(forward, [0.0, 1.0, 0.0]));
    let up = cross(right, forward);
    let focal = (camera.viewport_height / 2.0) / (camera.field_of_view.to_radians() / 2.0).tan();
    let half_width = (camera.viewport_width / 2.0).max(1e-9);
    let half_height = (camera.viewport_height / 2.0).max(1e-9);

    // clip.x = view.x * focal / half_width, clip.w = view.z, and depth maps
    // [near, far] onto [0, 1] - the same projection the CPU path divides
    // out by hand.
    let a = focal / half_width;
    let b = focal / half_height;
    let near = game3d::NEAR_PLANE;
    let far = FAR_PLANE;
    let c = far / (far - near);
    let d = -far * near / (far - near);

    let rows: [[f64; 4]; 4] = [
        [a * right[0], a * right[1], a * right[2], -a * dot(right, eye)],
        [b * up[0], b * up[1], b * up[2], -b * dot(up, eye)],
        [c * forward[0], c * forward[1], c * forward[2], -c * dot(forward, eye) + d],
        [forward[0], forward[1], forward[2], -dot(forward, eye)],
    ];
    for column in 0..4 {
        for row in 0..4 {
            block[column * 4 + row] = rows[row][column] as f32;
        }
    }

    let light_direction = [environment.light_x, environment.light_y, environment.light_z];
    if dot(light_direction, light_direction).sqrt() < 1e-9 {
        return Err("game3d_scene: the light direction cannot be all zeroes - it points toward the light, and `0, 1, 0` is overhead".to_string());
    }
    let light = normalized(light_direction);
    block[16] = light[0] as f32;
    block[17] = light[1] as f32;
    block[18] = light[2] as f32;
    block[19] = environment.ambient.clamp(0.0, 1.0) as f32;

    let light_color = super::game::parse_color(&environment.light_color).map_err(|_| format!("game3d_scene: `{}` is not a colour this understands", environment.light_color))?;
    block[20] = light_color.red();
    block[21] = light_color.green();
    block[22] = light_color.blue();
    block[23] = 0.0;

    let fog_on = environment.fog_far > environment.fog_near;
    if fog_on {
        let fog_color = super::game::parse_color(&environment.fog_color).map_err(|_| format!("game3d_scene: `{}` is not a colour this understands", environment.fog_color))?;
        block[24] = fog_color.red();
        block[25] = fog_color.green();
        block[26] = fog_color.blue();
    }
    block[27] = if fog_on { 1.0 } else { 0.0 };
    block[28] = environment.fog_near as f32;
    block[29] = environment.fog_far.max(environment.fog_near + 1e-6) as f32;
    block[32] = eye[0] as f32;
    block[33] = eye[1] as f32;
    block[34] = eye[2] as f32;
    block[35] = clock_seconds() as f32;
    return Ok(());
}

/// Appends one draw's instance data: the model matrix, the normal matrix
/// (the rotation with the scale divided back out, honest under non-uniform
/// scale), and the tint.
fn push_instance(instances: &mut Vec<f32>, draw: &game3d::GAME3D_Draw) -> Result<(), String> {
    let (sin_x, cos_x) = draw.rotation_x.sin_cos();
    let (sin_y, cos_y) = draw.rotation_y.sin_cos();
    let (sin_z, cos_z) = draw.rotation_z.sin_cos();
    let rotate_x = [[1.0, 0.0, 0.0], [0.0, cos_x, sin_x], [0.0, -sin_x, cos_x]];
    let rotate_y = [[cos_y, 0.0, -sin_y], [0.0, 1.0, 0.0], [sin_y, 0.0, cos_y]];
    let rotate_z = [[cos_z, sin_z, 0.0], [-sin_z, cos_z, 0.0], [0.0, 0.0, 1.0]];
    // Columns of the rotation applied x first, then y, then z.
    let rotation = multiply3(rotate_z, multiply3(rotate_y, rotate_x));

    let scale = [draw.scale_x, draw.scale_y, draw.scale_z];
    let safe = |value: f64| if value.abs() < 1e-9 { 1.0 } else { value };
    // Model columns are the rotation's columns stretched by the scale.
    for axis in 0..3 {
        for row in 0..3 {
            instances.push((rotation[axis][row] * scale[axis]) as f32);
        }
        instances.push(0.0);
    }
    instances.push(draw.position_x as f32);
    instances.push(draw.position_y as f32);
    instances.push(draw.position_z as f32);
    instances.push(1.0);
    // Normal matrix columns: the rotation with the scale divided out. The
    // first column's padding float carries the glow.
    for axis in 0..3 {
        for row in 0..3 {
            instances.push((rotation[axis][row] / safe(scale[axis])) as f32);
        }
        instances.push(if axis == 0 { draw.glow.clamp(0.0, 1.0) as f32 } else { 0.0 });
    }
    match draw.tint.is_empty() {
        true => instances.extend_from_slice(&[0.0, 0.0, 0.0, 0.0]),
        false => {
            let hex = draw.tint.strip_prefix('#').filter(|rest| rest.len() == 6);
            let parsed = hex.and_then(|rest| {
                let part = |from: usize| u8::from_str_radix(&rest[from..from + 2], 16).ok();
                return Some((part(0)?, part(2)?, part(4)?));
            });
            let Some((red, green, blue)) = parsed else {
                return Err(format!("game3d_scene: `{}` is not a #rrggbb colour", draw.tint));
            };
            instances.extend_from_slice(&[red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0, 1.0]);
        }
    }
    instances.extend_from_slice(&[draw.param_a as f32, draw.param_b as f32, draw.param_c as f32, draw.param_d as f32]);
    return Ok(());
}

/// Column-major 3x3 multiply: the columns of `second` carried through
/// `first`.
fn multiply3(first: [[f64; 3]; 3], second: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut out = [[0.0f64; 3]; 3];
    for column in 0..3 {
        for row in 0..3 {
            out[column][row] = first[0][row] * second[column][0] + first[1][row] * second[column][1] + first[2][row] * second[column][2];
        }
    }
    return out;
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

fn normalized(vector: [f64; 3]) -> [f64; 3] {
    let length = dot(vector, vector).sqrt().max(f64::MIN_POSITIVE);
    return [vector[0] / length, vector[1] / length, vector[2] / length];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtin_mesh_shaders_still_compile() {
        // The builtin lit shader is only ever parsed on a real device at run
        // time, so an edit to its WGSL would otherwise first fail on a
        // player's machine. The custom wrapper goes through the same check
        // with the smallest shader a program could write.
        let lit = format!("{}\n{}", MESH_SHADER_COMMON, MESH_SHADER_LIT_FS);
        let module = naga::front::wgsl::parse_str(&lit).expect("the builtin lit shader should parse");
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
            .validate(&module)
            .expect("the builtin lit shader should validate");
        validate_custom_shader("fn shade(surface: NAIL_Surface) -> vec4<f32> { return vec4<f32>(surface.color, 1.0); }")
            .expect("the custom wrapper should validate");
    }

    #[test]
    fn a_line_becomes_the_rectangle_its_stroke_covers() {
        let (origin, along, across) = line_quad([0.0, 0.0], [10.0, 0.0], 4.0).unwrap();
        assert_eq!(origin, [0.0, -2.0], "the quad starts half the thickness to one side");
        assert_eq!(along, [10.0, 0.0]);
        assert_eq!(across, [0.0, 4.0], "and spans the whole thickness to the other");
        assert!(line_quad([1.0, 1.0], [1.0, 1.0], 4.0).is_none(), "a zero-length line draws nothing");
        assert!(line_quad([0.0, 0.0], [10.0, 0.0], 0.0).is_none(), "a zero-width line draws nothing");
    }

    #[test]
    fn colors_premultiply_the_way_the_blender_expects() {
        let opaque = premultiplied("#ff0000").unwrap();
        assert_eq!(opaque, [1.0, 0.0, 0.0, 1.0]);
        let half = premultiplied("#ff000080").unwrap();
        assert!((half[3] - 128.0 / 255.0).abs() < 1e-3);
        assert!((half[0] - half[3]).abs() < 1e-3, "red is scaled by alpha");
        assert!(premultiplied("nonsense").is_err());
        let text = premultiplied_opaque("#336699").unwrap();
        assert!((text[3] - 1.0).abs() < 1e-6, "text lets glyph coverage be the alpha");
    }

    #[test]
    fn batches_only_break_when_the_texture_changes() {
        let mut batches: Vec<Batch2D> = Vec::new();
        extend_batch(&mut batches, BindSource::Atlas, 0);
        extend_batch(&mut batches, BindSource::Atlas, SHAPE2D_FLOATS);
        extend_batch(&mut batches, BindSource::Sprite(3), SHAPE2D_FLOATS * 2);
        extend_batch(&mut batches, BindSource::Atlas, SHAPE2D_FLOATS * 3);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].count, 2);
        assert_eq!(batches[1].count, 1);
        assert_eq!(batches[2].byte_offset, (SHAPE2D_FLOATS * 3 * 4) as u64);
    }

    #[test]
    fn one_instance_is_five_vec4s() {
        let mut floats = Vec::new();
        push_quad(&mut floats, KIND_CIRCLE, [10.0, 20.0], [8.0, 0.0], [0.0, 8.0], [0.0, 0.0, 1.0, 1.0], [1.0, 1.0, 1.0, 1.0], [4.0, 0.0, 0.0, 0.0]);
        assert_eq!(floats.len(), SHAPE2D_FLOATS);
        assert_eq!(floats[0], KIND_CIRCLE);
        assert_eq!((floats[1], floats[2]), (10.0, 20.0));
        assert_eq!(floats[16], 4.0, "the radius rides in the params");
    }
}
