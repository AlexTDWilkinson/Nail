//! Three dimensions: a camera, meshes, scenes, and the same 2D frame
//! everything else draws into.
//!
//! There are two ways in. `game3d_mesh` is the original: it takes a camera
//! and a loaded mesh and returns an array of plain `GAME_Shape` triangles -
//! projected, lit and depth-sorted on the CPU, ready to go into a
//! `GAME_Frame` next to any 2D shape. `game3d_scene` is the whole-scene way:
//! it takes a camera, an environment (light, ambient, fog) and an array of
//! `GAME3D_Draw` placements, and returns a single shape the backend renders
//! with a real depth buffer on the graphics card when one exists, or expands
//! into painter-ordered triangles right here when one does not. Same
//! program, both worlds.
//!
//! Models load from glTF, the format every 3D tool exports. On a real
//! machine `game3d_mesh_load` reads the file from disk, in the browser build
//! the same call fetches the same path as a URL. Textures are not read -
//! triangles take their material's base colour and directional light shades
//! them - so low-poly models with coloured materials look best, which suits
//! the whole aesthetic. Meshes can also be generated (`game3d_mesh_cube`,
//! `game3d_mesh_sphere`, `game3d_mesh_ground`, ...) or built triangle by
//! triangle from a flat array of numbers (`game3d_mesh_from_triangles`),
//! which is the raw material for terrain, voxels, or anything procedural.
//!
//! The camera looks from its position toward its target with y up. Loaded
//! meshes are normalised - centred at the origin, longest side scaled to
//! one unit - so a model from anywhere on the internet shows up on screen
//! instead of being a thousand units off in the dark.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};

use super::game::GAME_Shape;

/// Where the eye is, what it looks at, and how much it sees. `field_of_view`
/// is vertical, in degrees, and 60 is a sane default. The viewport is the
/// size of the frame being drawn into, in pixels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAME3D_Camera {
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub target_x: f64,
    pub target_y: f64,
    pub target_z: f64,
    pub field_of_view: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
}

/// One mesh placed in the world: where it stands, how it is spun about each
/// axis (radians, applied x then y then z), how it is stretched along each
/// axis, an optional repaint, how much it glows, which loaded shader
/// paints it (0 is the builtin one), and four numbers for that shader to
/// read. Built by `game3d_draw` and reshaped by the `game3d_draw_*`
/// functions rather than written out by hand, so new fields can arrive
/// without breaking a single program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAME3D_Draw {
    pub mesh: i64,
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub rotation_x: f64,
    pub rotation_y: f64,
    pub rotation_z: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub scale_z: f64,
    pub tint: String,
    pub glow: f64,
    pub shader: i64,
    pub param_a: f64,
    pub param_b: f64,
    pub param_c: f64,
    pub param_d: f64,
}

/// A loaded surface shader, handed back by `game3d_shader` and accepted by
/// `game3d_draw_shaded`. Its own type, so a mesh handle can never be passed
/// where a shader belongs and the mistake is the type checker's to catch.
/// PartialEq because programs keep it in their state structs, which derive
/// comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GAME3D_Shader {
    pub handle: i64,
}

/// The light and the air of a scene. The light direction points TOWARD the
/// sun - an overhead light is `0, 1, 0`. `ambient` is the floor brightness a
/// face gets with no light on it at all, 0 to 1. Fog blends everything
/// toward `fog_color` between `fog_near` and `fog_far` world units from the
/// camera, and a `fog_far` of 0 means no fog. Built by `game3d_environment`
/// and reshaped by the `game3d_environment_*` functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAME3D_Environment {
    pub light_x: f64,
    pub light_y: f64,
    pub light_z: f64,
    pub light_color: String,
    pub ambient: f64,
    pub fog_color: String,
    pub fog_near: f64,
    pub fog_far: f64,
}

/// Where a world point lands on the screen. `depth` is how far in front of
/// the camera it sits, in world units, and `visible` says whether it is both
/// in front of the camera and inside the viewport - a label drawn at x, y
/// when `visible` is true follows its object exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAME3D_ScreenPoint {
    pub screen_x: f64,
    pub screen_y: f64,
    pub depth: f64,
    pub visible: bool,
}

/// A half-line through the world: the camera's own position and the
/// direction under a screen pixel. What mouse picking and shooting both
/// need. `direction` has length one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAME3D_Ray {
    pub origin_x: f64,
    pub origin_y: f64,
    pub origin_z: f64,
    pub direction_x: f64,
    pub direction_y: f64,
    pub direction_z: f64,
}

/// One triangle of a mesh, in the mesh's own space.
#[derive(Clone)]
pub(crate) struct MeshTriangle {
    pub(crate) corners: [[f32; 3]; 3],
    pub(crate) color: (u8, u8, u8),
}

/// A BSP tree over a mesh's triangles. Traversed by camera position it
/// hands back triangles in exact far-to-near order, which no depth-sorting
/// heuristic can promise: stacked parallel planes and walls standing on
/// floors always have some angle where a single sort key orders them
/// wrongly. Spanning triangles are split at build time so every triangle
/// lies wholly on one side of every plane above it.
struct BspNode {
    plane_point: [f64; 3],
    plane_normal: [f64; 3],
    coplanar: Vec<MeshTriangle>,
    front: Option<Box<BspNode>>,
    back: Option<Box<BspNode>>,
}

const BSP_EPSILON: f64 = 1e-4;

/// How close a point may come to the camera plane before it is dropped.
/// Shared by the CPU projector and the GPU renderer's near plane, so the
/// two paths agree about what is visible.
pub(crate) const NEAR_PLANE: f64 = 0.05;

fn corner_f64(corner: [f32; 3]) -> [f64; 3] {
    return [corner[0] as f64, corner[1] as f64, corner[2] as f64];
}

fn triangle_plane(triangle: &MeshTriangle) -> Option<([f64; 3], [f64; 3])> {
    let one = corner_f64(triangle.corners[0]);
    let two = corner_f64(triangle.corners[1]);
    let three = corner_f64(triangle.corners[2]);
    let normal = cross(subtract(two, one), subtract(three, one));
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if length < 1e-12 {
        return None;
    }
    return Some((one, [normal[0] / length, normal[1] / length, normal[2] / length]));
}

/// Clips a triangle to one side of a plane. `keep_front` picks the side.
/// The result is the surviving polygon fan-triangulated, so zero, one or
/// two triangles.
fn clip_triangle(triangle: &MeshTriangle, distances: [f64; 3], keep_front: bool) -> Vec<MeshTriangle> {
    let mut polygon: Vec<[f64; 3]> = Vec::with_capacity(4);
    for index in 0..3 {
        let next = (index + 1) % 3;
        let here = corner_f64(triangle.corners[index]);
        let there = corner_f64(triangle.corners[next]);
        let here_distance = if keep_front { distances[index] } else { -distances[index] };
        let there_distance = if keep_front { distances[next] } else { -distances[next] };
        if here_distance >= 0.0 {
            polygon.push(here);
        }
        if (here_distance >= 0.0) != (there_distance >= 0.0) {
            let along = here_distance / (here_distance - there_distance);
            polygon.push([
                here[0] + (there[0] - here[0]) * along,
                here[1] + (there[1] - here[1]) * along,
                here[2] + (there[2] - here[2]) * along,
            ]);
        }
    }
    let as_f32 = |point: [f64; 3]| [point[0] as f32, point[1] as f32, point[2] as f32];
    let mut out = Vec::new();
    for fan in 1..polygon.len().saturating_sub(1) {
        out.push(MeshTriangle { corners: [as_f32(polygon[0]), as_f32(polygon[fan]), as_f32(polygon[fan + 1])], color: triangle.color });
    }
    return out;
}

fn build_bsp(triangles: Vec<MeshTriangle>) -> Option<Box<BspNode>> {
    let (plane_point, plane_normal) = triangles.iter().find_map(triangle_plane)?;
    let mut coplanar = Vec::new();
    let mut front_side = Vec::new();
    let mut back_side = Vec::new();
    for triangle in triangles {
        let distances = [
            dot(subtract(corner_f64(triangle.corners[0]), plane_point), plane_normal),
            dot(subtract(corner_f64(triangle.corners[1]), plane_point), plane_normal),
            dot(subtract(corner_f64(triangle.corners[2]), plane_point), plane_normal),
        ];
        let all_front = distances.iter().all(|d| *d >= -BSP_EPSILON);
        let all_back = distances.iter().all(|d| *d <= BSP_EPSILON);
        if all_front && all_back {
            coplanar.push(triangle);
        } else if all_front {
            front_side.push(triangle);
        } else if all_back {
            back_side.push(triangle);
        } else {
            front_side.extend(clip_triangle(&triangle, distances, true));
            back_side.extend(clip_triangle(&triangle, distances, false));
        }
    }
    return Some(Box::new(BspNode { plane_point, plane_normal, coplanar, front: build_bsp_vec(front_side), back: build_bsp_vec(back_side) }));
}

fn build_bsp_vec(triangles: Vec<MeshTriangle>) -> Option<Box<BspNode>> {
    if triangles.is_empty() {
        return None;
    }
    return build_bsp(triangles);
}

/// Far side first, this plane's own triangles, then the near side: exact
/// painter's order for wherever the eye is.
fn traverse_bsp<'tree>(node: &'tree BspNode, eye: [f64; 3], out: &mut Vec<&'tree MeshTriangle>) {
    let side = dot(subtract(eye, node.plane_point), node.plane_normal);
    let (far, near) = if side >= 0.0 { (&node.back, &node.front) } else { (&node.front, &node.back) };
    if let Some(child) = far {
        traverse_bsp(child, eye, out);
    }
    out.extend(node.coplanar.iter());
    if let Some(child) = near {
        traverse_bsp(child, eye, out);
    }
}

/// One stored mesh: the triangles as loaded (what the graphics card gets),
/// their bounding box (what picking tests against), and the BSP tree the
/// CPU fallback sorts by, built the first time it is actually needed so a
/// program that always renders on the graphics card never pays for it.
struct StoredMesh {
    raw: Vec<MeshTriangle>,
    low: [f32; 3],
    high: [f32; 3],
    bsp: Option<Box<BspNode>>,
}

impl StoredMesh {
    fn ensure_bsp(&mut self) -> &BspNode {
        if self.bsp.is_none() {
            self.bsp = build_bsp(self.raw.clone());
        }
        // store_mesh proved at least one triangle has area, so the build
        // cannot come back empty.
        return self.bsp.as_deref().expect("game3d: a stored mesh lost its triangles");
    }
}

fn meshes() -> &'static Mutex<HashMap<i64, StoredMesh>> {
    static MESHES: OnceLock<Mutex<HashMap<i64, StoredMesh>>> = OnceLock::new();
    return MESHES.get_or_init(|| Mutex::new(HashMap::new()));
}

static NEXT_MESH: AtomicI64 = AtomicI64::new(1);

/// The shader store: WGSL the program loaded, already validated, waiting for
/// the renderer to build a pipeline from each. Handle 0 is never in here -
/// it means the builtin shading.
fn shaders() -> &'static Mutex<HashMap<i64, String>> {
    static SHADERS: OnceLock<Mutex<HashMap<i64, String>>> = OnceLock::new();
    return SHADERS.get_or_init(|| Mutex::new(HashMap::new()));
}

static NEXT_SHADER: AtomicI64 = AtomicI64::new(1);

/// Loads a custom surface shader from WGSL source and hands back its handle.
/// The source must define one function, `fn shade(surface: NAIL_Surface) ->
/// vec4<f32>`, plus any helpers it wants. The engine keeps owning the vertex
/// stage, instancing and fog - the shader only turns a surface into a
/// colour. The source is compiled here, once, so a broken shader fails at
/// load with the compiler's real error instead of at first draw.
pub fn shader(source: String) -> Result<GAME3D_Shader, String> {
    super::game_gpu::validate_custom_shader(&source)?;
    let handle = NEXT_SHADER.fetch_add(1, Ordering::Relaxed);
    shaders().lock().map_err(|_| "game3d_shader: the shader store is poisoned".to_string())?.insert(handle, source);
    return Ok(GAME3D_Shader { handle });
}

/// The stored WGSL behind a handle, for the renderer building its pipeline.
pub(crate) fn shader_source(handle: i64) -> Result<String, String> {
    let store = shaders().lock().map_err(|_| "game3d_scene: the shader store is poisoned".to_string())?;
    return store.get(&handle).cloned().ok_or_else(|| format!("game3d_scene: a draw refers to shader {} but no shader with that number was loaded", handle));
}

fn store_mesh(triangles: Vec<MeshTriangle>) -> Result<i64, String> {
    if !triangles.iter().any(|triangle| triangle_plane(triangle).is_some()) {
        return Err("game3d: the mesh has no triangles with any area".to_string());
    }
    let mut low = [f32::MAX; 3];
    let mut high = [f32::MIN; 3];
    for triangle in &triangles {
        for corner in &triangle.corners {
            for axis in 0..3 {
                low[axis] = low[axis].min(corner[axis]);
                high[axis] = high[axis].max(corner[axis]);
            }
        }
    }
    let handle = NEXT_MESH.fetch_add(1, Ordering::Relaxed);
    meshes().lock().map_err(|_| "game3d: the mesh store is poisoned".to_string())?.insert(handle, StoredMesh { raw: triangles, low, high, bsp: None });
    return Ok(handle);
}

/// Centres a soup of triangles on the origin and scales its longest side to
/// one unit, so `scale` in a draw means the same thing for any model.
fn normalize(triangles: &mut Vec<MeshTriangle>) {
    let mut low = [f32::MAX; 3];
    let mut high = [f32::MIN; 3];
    for triangle in triangles.iter() {
        for corner in &triangle.corners {
            for axis in 0..3 {
                low[axis] = low[axis].min(corner[axis]);
                high[axis] = high[axis].max(corner[axis]);
            }
        }
    }
    let middle = [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0, (low[2] + high[2]) / 2.0];
    let longest = (high[0] - low[0]).max(high[1] - low[1]).max(high[2] - low[2]).max(f32::MIN_POSITIVE);
    for triangle in triangles.iter_mut() {
        for corner in triangle.corners.iter_mut() {
            for axis in 0..3 {
                corner[axis] = (corner[axis] - middle[axis]) / longest;
            }
        }
    }
}

/// Pulls every triangle out of a glTF document, with its material's base
/// colour. Textures and animations are left where they are.
fn triangles_from_gltf(bytes: &[u8], what: &str) -> Result<Vec<MeshTriangle>, String> {
    let (document, buffers, _images) = gltf::import_slice(bytes).map_err(|e| format!("game3d_mesh_load: {} is not a glTF file this understands: {}", what, e))?;
    let mut triangles = Vec::new();
    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let base = primitive.material().pbr_metallic_roughness().base_color_factor();
            let color = ((base[0] * 255.0) as u8, (base[1] * 255.0) as u8, (base[2] * 255.0) as u8);
            let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|data| &data.0[..]));
            let Some(positions) = reader.read_positions() else { continue };
            let positions: Vec<[f32; 3]> = positions.collect();
            let corner_of = |index: usize| positions.get(index).copied();
            match reader.read_indices() {
                Some(indices) => {
                    let indices: Vec<u32> = indices.into_u32().collect();
                    for triple in indices.chunks_exact(3) {
                        if let (Some(one), Some(two), Some(three)) = (corner_of(triple[0] as usize), corner_of(triple[1] as usize), corner_of(triple[2] as usize)) {
                            triangles.push(MeshTriangle { corners: [one, two, three], color });
                        }
                    }
                }
                None => {
                    for triple in positions.chunks_exact(3) {
                        triangles.push(MeshTriangle { corners: [triple[0], triple[1], triple[2]], color });
                    }
                }
            }
        }
    }
    if triangles.is_empty() {
        return Err(format!("game3d_mesh_load: {} contains no triangles", what));
    }
    let mut triangles = triangles;
    normalize(&mut triangles);
    return Ok(triangles);
}

/// Reads a glTF model and returns the number that names it from now on. On a
/// real machine the path is a file, in a browser it is fetched as a URL, so
/// the same program works in both worlds. Binary .glb files carry everything
/// in one file and are the form to reach for.
#[cfg(not(target_arch = "wasm32"))]
pub async fn mesh_load(path: String) -> Result<i64, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("game3d_mesh_load: could not read {}: {}", path, e))?;
    let triangles = triangles_from_gltf(&bytes, &path)?;
    return store_mesh(triangles);
}

/// The browser build fetches the path as a URL relative to the page.
#[cfg(target_arch = "wasm32")]
pub async fn mesh_load(path: String) -> Result<i64, String> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or_else(|| "game3d_mesh_load: there is no browser window to fetch from".to_string())?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(&path)).await.map_err(|_| format!("game3d_mesh_load: could not fetch {}", path))?;
    let response: web_sys::Response = response.dyn_into().map_err(|_| format!("game3d_mesh_load: could not fetch {}", path))?;
    if !response.ok() {
        return Err(format!("game3d_mesh_load: fetching {} answered status {}", path, response.status()));
    }
    let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer().map_err(|_| format!("game3d_mesh_load: {} gave no body", path))?)
        .await
        .map_err(|_| format!("game3d_mesh_load: could not read the body of {}", path))?;
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
    let triangles = triangles_from_gltf(&bytes, &path)?;
    return store_mesh(triangles);
}

/// A unit cube with a different shade on each face, generated rather than
/// loaded, so there is something to spin before any model has been found.
pub fn mesh_cube() -> Result<i64, String> {
    let corner = |x: f32, y: f32, z: f32| [x - 0.5, y - 0.5, z - 0.5];
    let mut triangles = Vec::new();
    let faces: [([f32; 3], [f32; 3], [f32; 3], [f32; 3], (u8, u8, u8)); 6] = [
        (corner(0.0, 0.0, 1.0), corner(1.0, 0.0, 1.0), corner(1.0, 1.0, 1.0), corner(0.0, 1.0, 1.0), (0xe5, 0x39, 0x35)),
        (corner(1.0, 0.0, 0.0), corner(0.0, 0.0, 0.0), corner(0.0, 1.0, 0.0), corner(1.0, 1.0, 0.0), (0x43, 0xa0, 0x47)),
        (corner(1.0, 0.0, 1.0), corner(1.0, 0.0, 0.0), corner(1.0, 1.0, 0.0), corner(1.0, 1.0, 1.0), (0x1e, 0x88, 0xe5)),
        (corner(0.0, 0.0, 0.0), corner(0.0, 0.0, 1.0), corner(0.0, 1.0, 1.0), corner(0.0, 1.0, 0.0), (0xfd, 0xd8, 0x35)),
        (corner(0.0, 1.0, 1.0), corner(1.0, 1.0, 1.0), corner(1.0, 1.0, 0.0), corner(0.0, 1.0, 0.0), (0xfb, 0x8c, 0x00)),
        (corner(0.0, 0.0, 0.0), corner(1.0, 0.0, 0.0), corner(1.0, 0.0, 1.0), corner(0.0, 0.0, 1.0), (0x8e, 0x24, 0xaa)),
    ];
    for (one, two, three, four, color) in faces {
        triangles.push(MeshTriangle { corners: [one, two, three], color });
        triangles.push(MeshTriangle { corners: [one, three, four], color });
    }
    return store_mesh(triangles);
}

/// Reads a colour the way every 2D shape does - `#rrggbb`, `#rgb`,
/// `#rrggbbaa` or a basic name - and keeps the three channels.
fn mesh_color(name: &str, who: &str) -> Result<(u8, u8, u8), String> {
    let color = super::game::parse_color(name).map_err(|_| format!("{}: `{}` is not a colour this understands - use `#rrggbb` or a basic name like `red`", who, name))?;
    return Ok(((color.red() * 255.0) as u8, (color.green() * 255.0) as u8, (color.blue() * 255.0) as u8));
}

/// A generated sphere of one colour, half a unit in radius so it fills the
/// same box a cube does. `bands` is how many horizontal slices it is built
/// from - 3 is a gem, 24 is smooth - clamped to something sane either way.
pub fn mesh_sphere(color: String, bands: i64) -> Result<i64, String> {
    let color = mesh_color(&color, "game3d_mesh_sphere")?;
    let bands = bands.clamp(3, 48) as usize;
    let segments = bands * 2;
    let point = |band: usize, segment: usize| -> [f32; 3] {
        let theta = std::f64::consts::PI * band as f64 / bands as f64;
        let phi = std::f64::consts::TAU * segment as f64 / segments as f64;
        return [(0.5 * theta.sin() * phi.cos()) as f32, (0.5 * theta.cos()) as f32, (0.5 * theta.sin() * phi.sin()) as f32];
    };
    let mut triangles = Vec::new();
    for band in 0..bands {
        for segment in 0..segments {
            let a = point(band, segment);
            let b = point(band + 1, segment);
            let c = point(band + 1, segment + 1);
            let d = point(band, segment + 1);
            // Wound so the face normal points out of the sphere.
            if band + 1 < bands {
                triangles.push(MeshTriangle { corners: [a, c, b], color });
            }
            if band > 0 {
                triangles.push(MeshTriangle { corners: [a, d, c], color });
            }
        }
    }
    return store_mesh(triangles);
}

/// A flat unit square of one colour, lying in the ground plane, visible
/// from above and below.
pub fn mesh_plane(color: String) -> Result<i64, String> {
    let color = mesh_color(&color, "game3d_mesh_plane")?;
    return store_mesh(plane_cell(-0.5, -0.5, 1.0, color));
}

/// One ground cell as two up-facing and two down-facing triangles, so the
/// ground never vanishes when the camera dips below it.
fn plane_cell(x: f32, z: f32, size: f32, color: (u8, u8, u8)) -> Vec<MeshTriangle> {
    let a = [x, 0.0, z];
    let b = [x, 0.0, z + size];
    let c = [x + size, 0.0, z + size];
    let d = [x + size, 0.0, z];
    return vec![
        MeshTriangle { corners: [a, b, c], color },
        MeshTriangle { corners: [a, c, d], color },
        MeshTriangle { corners: [a, c, b], color },
        MeshTriangle { corners: [a, d, c], color },
    ];
}

/// A unit checkerboard: `squares` cells along each side, alternating between
/// the two colours. Scale a draw of it up a hundred times and there is a
/// floor with visible perspective for free.
pub fn mesh_ground(color_a: String, color_b: String, squares: i64) -> Result<i64, String> {
    let first = mesh_color(&color_a, "game3d_mesh_ground")?;
    let second = mesh_color(&color_b, "game3d_mesh_ground")?;
    let cells = squares.clamp(1, 32) as usize;
    let size = 1.0f32 / cells as f32;
    let mut triangles = Vec::new();
    for row in 0..cells {
        for column in 0..cells {
            let color = if (row + column) % 2 == 0 { first } else { second };
            triangles.extend(plane_cell(-0.5 + column as f32 * size, -0.5 + row as f32 * size, size, color));
        }
    }
    return store_mesh(triangles);
}

/// A generated cylinder of one colour standing on the y axis, one unit tall
/// and half a unit in radius, with caps. `sides` is how many flat faces
/// stand in for the curve.
pub fn mesh_cylinder(color: String, sides: i64) -> Result<i64, String> {
    let color = mesh_color(&color, "game3d_mesh_cylinder")?;
    let count = sides.clamp(3, 64) as usize;
    let rim = |side: usize, y: f32| -> [f32; 3] {
        let angle = std::f64::consts::TAU * side as f64 / count as f64;
        return [(0.5 * angle.cos()) as f32, y, (0.5 * angle.sin()) as f32];
    };
    let mut triangles = Vec::new();
    for side in 0..count {
        let bottom_here = rim(side, -0.5);
        let bottom_there = rim(side + 1, -0.5);
        let top_here = rim(side, 0.5);
        let top_there = rim(side + 1, 0.5);
        // The wall, wound so its normal points away from the axis.
        triangles.push(MeshTriangle { corners: [bottom_here, top_there, bottom_there], color });
        triangles.push(MeshTriangle { corners: [bottom_here, top_here, top_there], color });
        // The caps, up on top and down underneath.
        triangles.push(MeshTriangle { corners: [[0.0, 0.5, 0.0], top_there, top_here], color });
        triangles.push(MeshTriangle { corners: [[0.0, -0.5, 0.0], bottom_here, bottom_there], color });
    }
    return store_mesh(triangles);
}

/// A mesh built straight from numbers: nine per triangle (three corners of
/// x, y, z) and one colour per triangle - `array_repeat` makes a single
/// colour into as many as needed. The corners are kept exactly as given, no
/// centring and no scaling, because whoever computes terrain knows where it
/// goes. This is the raw material of anything procedural.
pub fn mesh_from_triangles(positions: Vec<f64>, colors: Vec<String>) -> Result<i64, String> {
    if positions.is_empty() {
        return Err("game3d_mesh_from_triangles: there are no corners to build from".to_string());
    }
    if positions.len() % 9 != 0 {
        return Err(format!("game3d_mesh_from_triangles: positions holds {} numbers, which is not whole triangles - each triangle needs 9, three corners of x, y and z", positions.len()));
    }
    if positions.iter().any(|value| !value.is_finite()) {
        return Err("game3d_mesh_from_triangles: a corner is not a finite number".to_string());
    }
    let count = positions.len() / 9;
    if colors.len() != count {
        return Err(format!("game3d_mesh_from_triangles: {} triangles but {} colours - give exactly one colour per triangle, and array_repeat turns one colour into a whole mesh's worth", count, colors.len()));
    }
    let mut parsed = Vec::with_capacity(colors.len());
    for color in &colors {
        parsed.push(mesh_color(color, "game3d_mesh_from_triangles")?);
    }
    let mut triangles = Vec::with_capacity(count);
    for index in 0..count {
        let at = |offset: usize| positions[index * 9 + offset] as f32;
        triangles.push(MeshTriangle {
            corners: [[at(0), at(1), at(2)], [at(3), at(4), at(5)], [at(6), at(7), at(8)]],
            color: parsed[index],
        });
    }
    return store_mesh(triangles);
}

/// The default light, heading mostly down and a little sideways.
// Direction toward the light source, an overhead sun a little to the side,
// so faces pointing up catch the most of it. The shading dots face normals
// against this directly.
const LIGHT: [f64; 3] = [-0.45, 0.8, -0.35];

/// The default floor brightness for a face the light misses entirely.
const AMBIENT: f64 = 0.45;

/// How hard the sun pushes past the ambient floor. Above 1 on purpose: with
/// plain diffuse shading a face only reaches full brightness when it points
/// exactly at the sun, so in an ordinary scene nothing ever shows its true
/// colour and a white wall renders grey. Overdriving and clamping saturates
/// every face within about forty five degrees of the sun, where a white
/// material is actually white. game_gpu.rs carries the same 1.4 in the
/// builtin shader, and the two must move together.
const SUN_STRENGTH: f64 = 1.4;

fn parse_tint(color: &str) -> Option<(u8, u8, u8)> {
    let hex = color.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let part = |from: usize| u8::from_str_radix(&hex[from..from + 2], 16).ok();
    return Some((part(0)?, part(2)?, part(4)?));
}

/// The camera turned into something projections can use: a view basis and
/// the focal length that turns view space into pixels.
struct Projector {
    eye: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
    up: [f64; 3],
    focal: f64,
    half_width: f64,
    half_height: f64,
}

fn projector_of(camera: &GAME3D_Camera) -> Projector {
    let forward = normalized([camera.target_x - camera.position_x, camera.target_y - camera.position_y, camera.target_z - camera.position_z]);
    let right = normalized(cross(forward, [0.0, 1.0, 0.0]));
    let up = cross(right, forward);
    return Projector {
        eye: [camera.position_x, camera.position_y, camera.position_z],
        forward,
        right,
        up,
        focal: (camera.viewport_height / 2.0) / (camera.field_of_view.to_radians() / 2.0).tan(),
        half_width: camera.viewport_width / 2.0,
        half_height: camera.viewport_height / 2.0,
    };
}

impl Projector {
    /// A world point in view space: x across, y up, z into the screen.
    fn view_of(&self, world: [f64; 3]) -> [f64; 3] {
        let toward = subtract(world, self.eye);
        return [dot(toward, self.right), dot(toward, self.up), dot(toward, self.forward)];
    }

    /// A view-space point on the screen. Only meaningful in front of the
    /// near plane.
    fn screen_of(&self, view: [f64; 3]) -> [f64; 2] {
        return [self.half_width + view[0] * self.focal / view[2], self.half_height - view[1] * self.focal / view[2]];
    }
}

/// One draw's transform, precomputed: scale, then rotation about x, y and z
/// in that order, then the move into place.
struct Placement {
    position: [f64; 3],
    scale: [f64; 3],
    sin_x: f64,
    cos_x: f64,
    sin_y: f64,
    cos_y: f64,
    sin_z: f64,
    cos_z: f64,
}

fn placement_of(draw: &GAME3D_Draw) -> Placement {
    let (sin_x, cos_x) = draw.rotation_x.sin_cos();
    let (sin_y, cos_y) = draw.rotation_y.sin_cos();
    let (sin_z, cos_z) = draw.rotation_z.sin_cos();
    return Placement { position: [draw.position_x, draw.position_y, draw.position_z], scale: [draw.scale_x, draw.scale_y, draw.scale_z], sin_x, cos_x, sin_y, cos_y, sin_z, cos_z };
}

impl Placement {
    /// A mesh corner carried into the world.
    fn place(&self, corner: [f32; 3]) -> [f64; 3] {
        let x = corner[0] as f64 * self.scale[0];
        let y = corner[1] as f64 * self.scale[1];
        let z = corner[2] as f64 * self.scale[2];
        let (y, z) = (y * self.cos_x - z * self.sin_x, y * self.sin_x + z * self.cos_x);
        let (x, z) = (x * self.cos_y + z * self.sin_y, z * self.cos_y - x * self.sin_y);
        let (x, y) = (x * self.cos_z - y * self.sin_z, x * self.sin_z + y * self.cos_z);
        return [x + self.position[0], y + self.position[1], z + self.position[2]];
    }

    /// A world point carried back into the mesh's own space: the inverse of
    /// `place`, with a zero scale treated as tiny instead of dividing by it.
    fn unplace(&self, world: [f64; 3]) -> [f64; 3] {
        let x = world[0] - self.position[0];
        let y = world[1] - self.position[1];
        let z = world[2] - self.position[2];
        let (x, y) = (x * self.cos_z + y * self.sin_z, y * self.cos_z - x * self.sin_z);
        let (x, z) = (x * self.cos_y - z * self.sin_y, z * self.cos_y + x * self.sin_y);
        let (y, z) = (y * self.cos_x + z * self.sin_x, z * self.cos_x - y * self.sin_x);
        return [x / safe(self.scale[0]), y / safe(self.scale[1]), z / safe(self.scale[2])];
    }

    /// A direction carried back into the mesh's own space, for rays: the
    /// same inverse without the move.
    fn unplace_direction(&self, direction: [f64; 3]) -> [f64; 3] {
        let (x, y, z) = (direction[0], direction[1], direction[2]);
        let (x, y) = (x * self.cos_z + y * self.sin_z, y * self.cos_z - x * self.sin_z);
        let (x, z) = (x * self.cos_y - z * self.sin_y, z * self.cos_y + x * self.sin_y);
        let (y, z) = (y * self.cos_x + z * self.sin_x, z * self.cos_x - y * self.sin_x);
        return [x / safe(self.scale[0]), y / safe(self.scale[1]), z / safe(self.scale[2])];
    }
}

fn safe(scale: f64) -> f64 {
    return if scale.abs() < 1e-9 { 1.0 } else { scale };
}

/// The parsed, ready-to-use form of a `GAME3D_Environment`.
struct Lighting {
    light: [f64; 3],
    light_color: [f64; 3],
    ambient: f64,
    fog: Option<([f64; 3], f64, f64)>,
}

fn default_lighting() -> Lighting {
    return Lighting { light: normalized(LIGHT), light_color: [1.0, 1.0, 1.0], ambient: AMBIENT, fog: None };
}

fn lighting_of(environment: &GAME3D_Environment) -> Result<Lighting, String> {
    let direction = [environment.light_x, environment.light_y, environment.light_z];
    if dot(direction, direction).sqrt() < 1e-9 {
        return Err("game3d_scene: the light direction cannot be all zeroes - it points toward the light, and `0, 1, 0` is overhead".to_string());
    }
    let (red, green, blue) = mesh_color(&environment.light_color, "game3d_scene")?;
    let light_color = [red as f64 / 255.0, green as f64 / 255.0, blue as f64 / 255.0];
    let fog = if environment.fog_far > environment.fog_near {
        let (fog_red, fog_green, fog_blue) = mesh_color(&environment.fog_color, "game3d_scene")?;
        Some(([fog_red as f64 / 255.0, fog_green as f64 / 255.0, fog_blue as f64 / 255.0], environment.fog_near, environment.fog_far))
    } else {
        None
    };
    return Ok(Lighting { light: normalized(direction), light_color, ambient: environment.ambient.clamp(0.0, 1.0), fog });
}

/// Projects one placed mesh's triangles onto the screen, lit and fogged,
/// appending the survivors to `out` in the far-to-near order the BSP hands
/// them back.
fn project_mesh(projector: &Projector, lighting: &Lighting, placement: &Placement, tint: Option<(u8, u8, u8)>, glow: f64, ordered: &[&MeshTriangle], out: &mut Vec<GAME_Shape>) {
    let glow = glow.clamp(0.0, 1.0);
    for triangle in ordered {
        let mut world = [[0.0f64; 3]; 3];
        for (index, corner) in triangle.corners.iter().enumerate() {
            world[index] = placement.place(*corner);
        }

        // Flat shading wants the face normal in world space, which also
        // keeps it honest under a non-uniform scale.
        let edge_one = subtract(world[1], world[0]);
        let edge_two = subtract(world[2], world[0]);
        let normal = normalized(cross(edge_one, edge_two));
        let facing = dot(normal, lighting.light).max(0.0);
        let shaded = (lighting.ambient + (1.0 - lighting.ambient) * facing * SUN_STRENGTH).min(1.0);
        // A glowing mesh supplies its own light: glow pulls the shading
        // toward full brightness however the scene is lit.
        let brightness = shaded + (1.0 - shaded) * glow;

        let mut points = [[0.0f64; 2]; 3];
        let mut depth_sum = 0.0;
        let mut behind = false;
        for (index, corner) in world.iter().enumerate() {
            let view = projector.view_of(*corner);
            if view[2] < NEAR_PLANE {
                behind = true;
                break;
            }
            depth_sum += view[2];
            points[index] = projector.screen_of(view);
        }
        if behind {
            continue;
        }

        // Backface culling. glTF front faces wind counter-clockwise seen
        // from outside, and the screen's y grows downward, which flips the
        // sign of the winding - so on screen a front face comes out
        // clockwise, which is a negative cross product here.
        let winding = (points[1][0] - points[0][0]) * (points[2][1] - points[0][1]) - (points[1][1] - points[0][1]) * (points[2][0] - points[0][0]);
        if winding >= 0.0 {
            continue;
        }

        // Push every EDGE outward by half a pixel, corners mitered. The
        // rasterizer antialiases each edge, and two neighbours sharing an
        // edge both feather it, letting the background bleed through as a
        // hairline seam. Offsetting the edges themselves swallows the
        // feather however large or skinny the triangle is, which growing
        // vertices away from the centroid does not: on a huge half-quad
        // triangle the vertex directions run mostly along the diagonal and
        // the diagonal itself barely moves. Miters are capped so a
        // needle-thin triangle cannot grow a spike.
        let mut grown = points;
        for index in 0..3 {
            let here = points[index];
            let previous = points[(index + 2) % 3];
            let next = points[(index + 1) % 3];
            let outward = |from: [f64; 2], to: [f64; 2], opposite: [f64; 2]| -> [f64; 2] {
                let edge_x = to[0] - from[0];
                let edge_y = to[1] - from[1];
                let length = (edge_x * edge_x + edge_y * edge_y).sqrt().max(1e-9);
                let mut normal = [edge_y / length, -edge_x / length];
                if normal[0] * (opposite[0] - from[0]) + normal[1] * (opposite[1] - from[1]) > 0.0 {
                    normal = [-normal[0], -normal[1]];
                }
                return normal;
            };
            let normal_in = outward(previous, here, next);
            let normal_out = outward(here, next, previous);
            let sum = [normal_in[0] + normal_out[0], normal_in[1] + normal_out[1]];
            let denominator = (1.0 + (normal_in[0] * normal_out[0] + normal_in[1] * normal_out[1])).max(0.2);
            grown[index] = [here[0] + 0.5 * sum[0] / denominator, here[1] + 0.5 * sum[1] / denominator];
        }

        let (red, green, blue) = tint.unwrap_or(triangle.color);
        let mut channels = [red as f64 / 255.0 * brightness, green as f64 / 255.0 * brightness, blue as f64 / 255.0 * brightness];
        for (channel, light) in channels.iter_mut().zip(lighting.light_color) {
            *channel *= light;
        }
        if let Some((fog_color, fog_near, fog_far)) = lighting.fog {
            let depth = depth_sum / 3.0;
            // Glow burns through fog in proportion, the way headlights and
            // the sun cut through real haze while the hills behind them
            // vanish. The GPU shaders in game_gpu.rs do the same.
            let thickness = ((depth - fog_near) / (fog_far - fog_near)).clamp(0.0, 1.0) * (1.0 - glow);
            for (channel, fog) in channels.iter_mut().zip(fog_color) {
                *channel = *channel * (1.0 - thickness) + fog * thickness;
            }
        }
        let as_hex = |value: f64| (value.clamp(0.0, 1.0) * 255.0) as u8;
        let color = format!("#{:02x}{:02x}{:02x}", as_hex(channels[0]), as_hex(channels[1]), as_hex(channels[2]));
        out.push(super::game::triangle(grown[0][0], grown[0][1], grown[1][0], grown[1][1], grown[2][0], grown[2][1], color));
    }
}

/// Projects a mesh through a camera into flat triangles: placed, spun around
/// y, scaled, lit by the default light, ordered far to near by walking the
/// mesh's BSP tree from the eye, and returned as ordinary shapes for the
/// frame. An empty `tint` keeps the mesh's own material colours, a
/// `#rrggbb` tint repaints every triangle.
pub fn mesh(camera: GAME3D_Camera, handle: i64, x: f64, y: f64, z: f64, rotation_y: f64, scale: f64, tint: String) -> Result<Vec<GAME_Shape>, String> {
    let tint = if tint.is_empty() { None } else { Some(parse_tint(&tint).ok_or_else(|| format!("game3d_mesh: `{}` is not a #rrggbb colour", tint))?) };
    let mut store = meshes().lock().map_err(|_| "game3d_mesh: the mesh store is poisoned".to_string())?;
    let Some(stored) = store.get_mut(&handle) else {
        return Err(format!("game3d_mesh: no mesh with the number {} was loaded", handle));
    };
    let projector = projector_of(&camera);
    let lighting = default_lighting();
    let placement = placement_of(&GAME3D_Draw { mesh: handle, position_x: x, position_y: y, position_z: z, rotation_x: 0.0, rotation_y, rotation_z: 0.0, scale_x: scale, scale_y: scale, scale_z: scale, tint: String::new(), glow: 0.0, shader: 0, param_a: 0.0, param_b: 0.0, param_c: 0.0, param_d: 0.0 });

    // The eye moved into the mesh's own space picks the BSP traversal
    // order. The tree hands triangles back exactly far-to-near, so they are
    // drawn as they come with no sorting.
    let eye_local = placement.unplace(projector.eye);
    let bsp = stored.ensure_bsp();
    let mut ordered: Vec<&MeshTriangle> = Vec::new();
    traverse_bsp(bsp, eye_local, &mut ordered);

    let mut shapes: Vec<GAME_Shape> = Vec::with_capacity(ordered.len());
    project_mesh(&projector, &lighting, &placement, tint, 0.0, &ordered, &mut shapes);
    return Ok(shapes);
}

/// A line between two points in the world, projected onto the frame. Court
/// edges, axes, trajectories - anywhere the third dimension needs a wire.
pub fn line(camera: GAME3D_Camera, x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64, thickness: f64, color: String) -> Vec<GAME_Shape> {
    let projector = projector_of(&camera);
    let project = |point: [f64; 3]| -> Option<[f64; 2]> {
        let view = projector.view_of(point);
        if view[2] < NEAR_PLANE {
            return None;
        }
        return Some(projector.screen_of(view));
    };
    let (Some(start), Some(end)) = (project([x1, y1, z1]), project([x2, y2, z2])) else {
        return Vec::new();
    };
    return vec![super::game::line(start[0], start[1], end[0], end[1], thickness, color)];
}

/// One mesh placed at x, y, z with no spin, its own size and its own
/// colours. The other `game3d_draw_*` functions take it from there.
pub fn draw(mesh: i64, position_x: f64, position_y: f64, position_z: f64) -> GAME3D_Draw {
    return GAME3D_Draw { mesh, position_x, position_y, position_z, rotation_x: 0.0, rotation_y: 0.0, rotation_z: 0.0, scale_x: 1.0, scale_y: 1.0, scale_z: 1.0, tint: String::new(), glow: 0.0, shader: 0, param_a: 0.0, param_b: 0.0, param_c: 0.0, param_d: 0.0 };
}

/// The same draw lit from within: at 0 the light shades it like everything
/// else, at 1 it shows its full colour whatever the light does - a sun, a
/// lamp, lava. Fog still dims it with distance, which is what keeps a
/// glowing thing looking far away.
pub fn draw_glowing(draw: GAME3D_Draw, glow: f64) -> GAME3D_Draw {
    let mut draw = draw;
    draw.glow = glow;
    return draw;
}

/// The same draw painted by a shader loaded with `game3d_shader` instead of
/// the builtin lighting. The engine still places, projects and fogs it - the
/// shader only decides the surface's colour.
pub fn draw_shaded(draw: GAME3D_Draw, shader: GAME3D_Shader) -> GAME3D_Draw {
    let mut draw = draw;
    draw.shader = shader.handle;
    return draw;
}

/// Four numbers handed to the draw's custom shader as `surface.params`, so
/// one shader serves many draws: the same water rougher here and calmer
/// there, the same fire hotter and colder. Draws sharing a mesh and a
/// shader still batch whatever their params, the numbers ride the instance.
pub fn draw_shader_params(draw: GAME3D_Draw, param_a: f64, param_b: f64, param_c: f64, param_d: f64) -> GAME3D_Draw {
    let mut draw = draw;
    draw.param_a = param_a;
    draw.param_b = param_b;
    draw.param_c = param_c;
    draw.param_d = param_d;
    return draw;
}

/// The same draw spun to the given angles, radians, applied about x then y
/// then z. Spinning about y alone is the usual turntable.
pub fn draw_rotated(draw: GAME3D_Draw, rotation_x: f64, rotation_y: f64, rotation_z: f64) -> GAME3D_Draw {
    let mut draw = draw;
    draw.rotation_x = rotation_x;
    draw.rotation_y = rotation_y;
    draw.rotation_z = rotation_z;
    return draw;
}

/// The same draw stretched to the given size along each axis. The same
/// number three times scales evenly, different numbers squash and stretch,
/// and a ground mesh scaled by a hundred is a floor.
pub fn draw_scaled(draw: GAME3D_Draw, scale_x: f64, scale_y: f64, scale_z: f64) -> GAME3D_Draw {
    let mut draw = draw;
    draw.scale_x = scale_x;
    draw.scale_y = scale_y;
    draw.scale_z = scale_z;
    return draw;
}

/// The same draw repainted: every triangle takes this `#rrggbb` colour
/// instead of the mesh's own. An empty string goes back to the mesh's
/// colours.
pub fn draw_tinted(draw: GAME3D_Draw, tint: String) -> GAME3D_Draw {
    let mut draw = draw;
    draw.tint = tint;
    return draw;
}

/// The default environment: an overhead sun a little to the side, white
/// light, a sensible ambient floor, and no fog. Nail structs have no
/// default field values, so this saves spelling all eight out - a custom
/// sky writes the `GAME3D_Environment` literal instead, the way a camera
/// is written.
pub fn default_environment() -> GAME3D_Environment {
    return GAME3D_Environment {
        light_x: LIGHT[0],
        light_y: LIGHT[1],
        light_z: LIGHT[2],
        light_color: "#ffffff".to_string(),
        ambient: AMBIENT,
        fog_color: "#000000".to_string(),
        fog_near: 0.0,
        fog_far: 0.0,
    };
}

/// Everything one scene shape refers to, parked here between `view`
/// returning and the backend drawing.
pub(crate) struct SceneData {
    pub(crate) camera: GAME3D_Camera,
    pub(crate) environment: GAME3D_Environment,
    pub(crate) draws: Vec<GAME3D_Draw>,
}

fn scenes() -> &'static Mutex<HashMap<i64, SceneData>> {
    static SCENES: OnceLock<Mutex<HashMap<i64, SceneData>>> = OnceLock::new();
    return SCENES.get_or_init(|| Mutex::new(HashMap::new()));
}

static NEXT_SCENE: AtomicI64 = AtomicI64::new(1);

/// A whole 3D scene as one shape for the frame: these meshes, placed like
/// this, seen through this camera, lit like that. On a machine with a
/// graphics card the backend renders it there, depth-buffered, and 2D
/// shapes before it in the frame stay under it while shapes after it draw
/// over it - which is exactly where a HUD goes. Without one it becomes
/// depth-sorted triangles on the spot.
pub fn scene(camera: GAME3D_Camera, environment: GAME3D_Environment, draws: Vec<GAME3D_Draw>) -> GAME_Shape {
    let handle = match scenes().lock() {
        Ok(mut store) => {
            let handle = NEXT_SCENE.fetch_add(1, Ordering::Relaxed);
            store.insert(handle, SceneData { camera, environment, draws });
            handle
        }
        // A poisoned store cannot hold the scene. Handle 0 is never issued,
        // so drawing this shape reports a missing scene instead of drawing
        // someone else's.
        Err(_) => 0,
    };
    let mut shape = super::game::blank("scene3d");
    shape.sprite = handle;
    return shape;
}

/// Takes one scene out of the store for drawing. Each scene is drawn once,
/// so taking it also cleans it up.
pub(crate) fn take_scene(handle: i64) -> Option<SceneData> {
    return scenes().lock().ok()?.remove(&handle);
}

/// Clears whatever scenes a frame made but never put in a shape, called
/// after each present so the store cannot grow across frames.
pub(crate) fn sweep_scenes() {
    if let Ok(mut store) = scenes().lock() {
        store.clear();
    }
}

/// The CPU fallback for a scene: every draw expanded to lit, fogged,
/// painter-ordered triangles. Draws are ordered far to near by their
/// anchor points, and within each draw the mesh's BSP tree orders exactly.
pub(crate) fn expand_scene(data: &SceneData) -> Result<Vec<GAME_Shape>, String> {
    let projector = projector_of(&data.camera);
    let lighting = lighting_of(&data.environment)?;

    let mut order: Vec<usize> = (0..data.draws.len()).collect();
    let distance_of = |index: usize| {
        let draw = &data.draws[index];
        let apart = subtract([draw.position_x, draw.position_y, draw.position_z], projector.eye);
        return dot(apart, apart);
    };
    order.sort_by(|a, b| distance_of(*b).partial_cmp(&distance_of(*a)).unwrap_or(std::cmp::Ordering::Equal));

    let mut store = meshes().lock().map_err(|_| "game3d_scene: the mesh store is poisoned".to_string())?;
    let mut shapes: Vec<GAME_Shape> = Vec::new();
    for index in order {
        let draw = &data.draws[index];
        let tint = if draw.tint.is_empty() { None } else { Some(parse_tint(&draw.tint).ok_or_else(|| format!("game3d_scene: `{}` is not a #rrggbb colour", draw.tint))?) };
        let Some(stored) = store.get_mut(&draw.mesh) else {
            return Err(format!("game3d_scene: a draw refers to mesh {} but no mesh with that number was loaded", draw.mesh));
        };
        let placement = placement_of(draw);
        let eye_local = placement.unplace(projector.eye);
        let bsp = stored.ensure_bsp();
        let mut ordered: Vec<&MeshTriangle> = Vec::new();
        traverse_bsp(bsp, eye_local, &mut ordered);
        project_mesh(&projector, &lighting, &placement, tint, draw.glow, &ordered, &mut shapes);
    }
    return Ok(shapes);
}

/// Where a world point lands on the screen, and whether it is really there
/// to see. This is how a name tag, a health bar or a damage number follows
/// something that lives in the 3D world.
pub fn project(camera: GAME3D_Camera, x: f64, y: f64, z: f64) -> GAME3D_ScreenPoint {
    let projector = projector_of(&camera);
    let view = projector.view_of([x, y, z]);
    if view[2] < NEAR_PLANE {
        return GAME3D_ScreenPoint { screen_x: 0.0, screen_y: 0.0, depth: view[2], visible: false };
    }
    let screen = projector.screen_of(view);
    let visible = screen[0] >= 0.0 && screen[0] <= camera.viewport_width && screen[1] >= 0.0 && screen[1] <= camera.viewport_height;
    return GAME3D_ScreenPoint { screen_x: screen[0], screen_y: screen[1], depth: view[2], visible };
}

/// The ray under a screen pixel: the camera's position and the direction,
/// length one, that pixel looks along. The raw material for custom picking
/// and aiming.
pub fn ray(camera: GAME3D_Camera, screen_x: f64, screen_y: f64) -> GAME3D_Ray {
    let projector = projector_of(&camera);
    let across = (screen_x - projector.half_width) / projector.focal;
    let upward = (projector.half_height - screen_y) / projector.focal;
    let direction = normalized([
        projector.forward[0] + projector.right[0] * across + projector.up[0] * upward,
        projector.forward[1] + projector.right[1] * across + projector.up[1] * upward,
        projector.forward[2] + projector.right[2] * across + projector.up[2] * upward,
    ]);
    return GAME3D_Ray {
        origin_x: projector.eye[0],
        origin_y: projector.eye[1],
        origin_z: projector.eye[2],
        direction_x: direction[0],
        direction_y: direction[1],
        direction_z: direction[2],
    };
}

/// Which draw a ray hits: the index into `draws` of the nearest mesh whose
/// bounding box it passes through, or an error naming the miss. With the
/// ray from `game3d_ray` under the mouse, this is clicking on a unit.
pub fn pick(pointing: GAME3D_Ray, draws: Vec<GAME3D_Draw>) -> Result<i64, String> {
    let origin = [pointing.origin_x, pointing.origin_y, pointing.origin_z];
    let direction = [pointing.direction_x, pointing.direction_y, pointing.direction_z];
    let store = meshes().lock().map_err(|_| "game3d_pick: the mesh store is poisoned".to_string())?;
    let mut best: Option<(f64, usize)> = None;
    for (index, draw) in draws.iter().enumerate() {
        let Some(stored) = store.get(&draw.mesh) else { continue };
        let placement = placement_of(draw);
        let local_origin = placement.unplace(origin);
        let local_direction = placement.unplace_direction(direction);
        // A slab test against the mesh's box. The ray parameter survives
        // the transform into mesh space because the transform is affine,
        // so `enter` still measures world distances and picks the winner.
        let mut enter = f64::NEG_INFINITY;
        let mut exit = f64::INFINITY;
        let mut missed = false;
        for axis in 0..3 {
            let low = stored.low[axis] as f64;
            let high = stored.high[axis] as f64;
            if local_direction[axis].abs() < 1e-12 {
                if local_origin[axis] < low || local_origin[axis] > high {
                    missed = true;
                    break;
                }
                continue;
            }
            let toward_low = (low - local_origin[axis]) / local_direction[axis];
            let toward_high = (high - local_origin[axis]) / local_direction[axis];
            enter = enter.max(toward_low.min(toward_high));
            exit = exit.min(toward_low.max(toward_high));
        }
        if missed || enter > exit || exit < 0.0 {
            continue;
        }
        let along = enter.max(0.0);
        if best.map_or(true, |(closest, _)| along < closest) {
            best = Some((along, index));
        }
    }
    return match best {
        Some((_, index)) => Ok(index as i64),
        None => Err("game3d_pick: the ray passes every draw without touching one".to_string()),
    };
}

/// A mesh flattened for the graphics card: nine numbers per corner, three
/// corners per triangle - position, the face's normal, and the colour in
/// 0 to 1 - straight from the stored triangles, no BSP involved.
pub(crate) fn mesh_vertex_floats(handle: i64) -> Result<Vec<f32>, String> {
    let store = meshes().lock().map_err(|_| "game3d: the mesh store is poisoned".to_string())?;
    let Some(stored) = store.get(&handle) else {
        return Err(format!("game3d: no mesh with the number {} was loaded", handle));
    };
    let mut floats = Vec::with_capacity(stored.raw.len() * 27);
    for triangle in &stored.raw {
        let normal = match triangle_plane(triangle) {
            Some((_, normal)) => [normal[0] as f32, normal[1] as f32, normal[2] as f32],
            None => [0.0, 1.0, 0.0],
        };
        let color = [triangle.color.0 as f32 / 255.0, triangle.color.1 as f32 / 255.0, triangle.color.2 as f32 / 255.0];
        for corner in &triangle.corners {
            floats.extend_from_slice(corner);
            floats.extend_from_slice(&normal);
            floats.extend_from_slice(&color);
        }
    }
    return Ok(floats);
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    return [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
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

    fn test_camera() -> GAME3D_Camera {
        return GAME3D_Camera {
            position_x: 0.0,
            position_y: 0.0,
            position_z: -3.0,
            target_x: 0.0,
            target_y: 0.0,
            target_z: 0.0,
            field_of_view: 60.0,
            viewport_width: 800.0,
            viewport_height: 600.0,
        };
    }

    #[test]
    fn a_cube_in_front_of_the_camera_lands_on_screen() {
        let cube = mesh_cube().unwrap();
        let shapes = mesh(test_camera(), cube, 0.0, 0.0, 0.0, 0.3, 1.0, String::new()).unwrap();
        assert!(!shapes.is_empty(), "a visible cube projects to at least one triangle");
        // Culling has to remove roughly half of the twelve faces.
        assert!(shapes.len() <= 12);
        for shape in &shapes {
            assert_eq!(shape.kind, "triangle");
            assert!(shape.x_coordinate > 0.0 && shape.x_coordinate < 800.0, "the cube is around the middle of the screen");
        }
    }

    #[test]
    fn culling_keeps_the_face_toward_the_camera_not_the_one_behind() {
        // Head on, only one cube face can survive: the green one at the near
        // side. If culling is inverted, the red far face survives instead,
        // so the winner's colour is the whole test.
        let cube = mesh_cube().unwrap();
        let shapes = mesh(test_camera(), cube, 0.0, 0.0, 0.0, 0.0, 1.0, String::new()).unwrap();
        assert_eq!(shapes.len(), 2, "a head-on cube is exactly one face, two triangles");
        for shape in &shapes {
            let red = u8::from_str_radix(&shape.color[1..3], 16).unwrap();
            let green = u8::from_str_radix(&shape.color[3..5], 16).unwrap();
            assert!(green > red, "the near face is the green one, got {}", shape.color);
        }
    }

    #[test]
    fn a_mesh_behind_the_camera_disappears_instead_of_exploding() {
        let cube = mesh_cube().unwrap();
        let shapes = mesh(test_camera(), cube, 0.0, 0.0, -10.0, 0.0, 1.0, String::new()).unwrap();
        assert!(shapes.is_empty());
    }

    #[test]
    fn a_tint_repaints_and_a_bad_tint_is_an_error() {
        let cube = mesh_cube().unwrap();
        let tinted = mesh(test_camera(), cube, 0.0, 0.0, 0.0, 0.0, 1.0, "#ff0000".to_string()).unwrap();
        for shape in &tinted {
            assert!(shape.color.starts_with('#'));
        }
        assert!(mesh(test_camera(), cube, 0.0, 0.0, 0.0, 0.0, 1.0, "red".to_string()).is_err());
    }

    #[test]
    fn a_missing_mesh_is_a_named_error() {
        let missing = mesh(test_camera(), 999_999, 0.0, 0.0, 0.0, 0.0, 1.0, String::new());
        assert!(missing.is_err());
    }

    #[test]
    fn a_world_line_projects_to_one_screen_line() {
        let shapes = line(test_camera(), -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, "gray".to_string());
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].kind, "line");
    }

    fn single_draw_scene(handle: i64) -> Result<Vec<GAME_Shape>, String> {
        let data = SceneData { camera: test_camera(), environment: default_environment(), draws: vec![draw(handle, 0.0, 0.0, 0.0)] };
        return expand_scene(&data);
    }

    #[test]
    fn every_generated_mesh_faces_the_right_way() {
        // Each generator renders. A mesh wound inside out culls away
        // entirely and comes back empty, so non-empty is the whole winding
        // test. The round meshes face the head-on camera, the flat ones lie
        // edge-on to it and are looked down at instead.
        let sphere = mesh_sphere("red".to_string(), 8).unwrap();
        assert!(!single_draw_scene(sphere).unwrap().is_empty(), "the sphere projected to nothing");
        let cylinder = mesh_cylinder("#00ff00".to_string(), 12).unwrap();
        assert!(!single_draw_scene(cylinder).unwrap().is_empty(), "the cylinder projected to nothing");

        let mut looking_down = test_camera();
        looking_down.position_y = 2.0;
        for (handle, name) in [
            (mesh_ground("white".to_string(), "black".to_string(), 4).unwrap(), "ground"),
            (mesh_plane("blue".to_string()).unwrap(), "plane"),
        ] {
            let data = SceneData { camera: looking_down.clone(), environment: default_environment(), draws: vec![draw(handle, 0.0, 0.0, 0.0)] };
            assert!(!expand_scene(&data).unwrap().is_empty(), "the {} seen from above projected to nothing", name);
        }
    }

    #[test]
    fn a_sphere_is_round_enough_to_fill_its_box() {
        let sphere = mesh_sphere("red".to_string(), 16).unwrap();
        let store = meshes().lock().unwrap();
        let stored = store.get(&sphere).unwrap();
        for axis in 0..3 {
            assert!(stored.low[axis] < -0.45 && stored.high[axis] > 0.45, "axis {} spans {} to {}", axis, stored.low[axis], stored.high[axis]);
        }
    }

    #[test]
    fn from_triangles_validates_its_numbers() {
        assert!(mesh_from_triangles(vec![], vec!["red".to_string()]).is_err());
        assert!(mesh_from_triangles(vec![0.0; 8], vec!["red".to_string()]).is_err());
        assert!(mesh_from_triangles(vec![f64::NAN; 9], vec!["red".to_string()]).is_err());
        let flat = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 0.0, 0.0];
        assert!(mesh_from_triangles(flat, vec!["red".to_string()]).is_err(), "a zero-area mesh is refused");
        let one = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        assert!(mesh_from_triangles(one.clone(), vec!["red".to_string(), "blue".to_string()]).is_err(), "two colours for one triangle");
        assert!(mesh_from_triangles(one.clone(), vec!["nonsense".to_string()]).is_err());
        assert!(mesh_from_triangles(one, vec!["red".to_string()]).is_ok());
    }

    #[test]
    fn draw_builders_fill_what_they_say() {
        let placed = draw(7, 1.0, 2.0, 3.0);
        assert_eq!((placed.mesh, placed.position_x, placed.scale_x, placed.rotation_y), (7, 1.0, 1.0, 0.0));
        let spun = draw_rotated(placed.clone(), 0.1, 0.2, 0.3);
        assert_eq!((spun.rotation_x, spun.rotation_y, spun.rotation_z), (0.1, 0.2, 0.3));
        let stretched = draw_scaled(placed.clone(), 2.0, 3.0, 4.0);
        assert_eq!((stretched.scale_x, stretched.scale_y, stretched.scale_z), (2.0, 3.0, 4.0));
        let painted = draw_tinted(placed, "#123456".to_string());
        assert_eq!(painted.tint, "#123456");
        let air = default_environment();
        assert!(air.fog_far == 0.0 && air.ambient > 0.0);
    }

    #[test]
    fn a_scene_shape_holds_its_data_until_taken() {
        let cube = mesh_cube().unwrap();
        let shape = scene(test_camera(), default_environment(), vec![draw(cube, 0.0, 0.0, 0.0)]);
        assert_eq!(shape.kind, "scene3d");
        let data = take_scene(shape.sprite).expect("the scene is in the store");
        assert_eq!(data.draws.len(), 1);
        assert!(take_scene(shape.sprite).is_none(), "a scene is taken once");
        let swept = scene(test_camera(), default_environment(), vec![]);
        sweep_scenes();
        assert!(take_scene(swept.sprite).is_none(), "sweeping empties the store");
    }

    #[test]
    fn a_spun_scene_shows_the_far_face() {
        // Head on, the cube's near face is green. Spun half a turn about y,
        // the red face at the far side comes around to the front.
        let cube = mesh_cube().unwrap();
        let data = SceneData {
            camera: test_camera(),
            environment: default_environment(),
            draws: vec![draw_rotated(draw(cube, 0.0, 0.0, 0.0), 0.0, std::f64::consts::PI, 0.0)],
        };
        let shapes = expand_scene(&data).unwrap();
        assert_eq!(shapes.len(), 2);
        for shape in &shapes {
            let red = u8::from_str_radix(&shape.color[1..3], 16).unwrap();
            let green = u8::from_str_radix(&shape.color[3..5], 16).unwrap();
            assert!(red > green, "the face that came around is the red one, got {}", shape.color);
        }
    }

    #[test]
    fn a_glowing_mesh_ignores_the_dark() {
        // Almost no ambient and the light straight overhead: an unlit cube
        // seen head on is nearly black, a fully glowing one shows its own
        // colour at full strength whatever the light does.
        let cube = mesh_cube().unwrap();
        let mut dark = default_environment();
        dark.ambient = 0.02;
        dark.light_y = 1.0;
        dark.light_x = 0.0;
        dark.light_z = 0.0;
        let dim = SceneData { camera: test_camera(), environment: dark.clone(), draws: vec![draw(cube, 0.0, 0.0, 0.0)] };
        let lit = SceneData { camera: test_camera(), environment: dark, draws: vec![draw_glowing(draw(cube, 0.0, 0.0, 0.0), 1.0)] };
        // Every cube face's brightest channel is at least 0xa0, so whichever
        // face lands first, full emission pushes some channel high.
        let brightest = |shapes: &Vec<GAME_Shape>| (1..6).step_by(2).map(|from| u8::from_str_radix(&shapes[0].color[from..from + 2], 16).unwrap()).max().unwrap();
        let dim_peak = brightest(&expand_scene(&dim).unwrap());
        let lit_peak = brightest(&expand_scene(&lit).unwrap());
        assert!(dim_peak < 30, "unlit and facing away from the light is near black, got {}", dim_peak);
        assert!(lit_peak > 150, "glowing keeps its colour in the dark, got {}", lit_peak);
    }

    #[test]
    fn a_shader_loads_and_a_broken_one_names_its_error() {
        let good = shader("fn shade(surface: NAIL_Surface) -> vec4<f32> { return vec4<f32>(surface.color, 1.0); }".to_string()).unwrap();
        assert!(good.handle > 0, "a loaded shader gets a real handle");
        assert!(shader_source(good.handle).unwrap().contains("shade"), "the source waits for the renderer");

        let broken = shader("fn shade(surface: NAIL_Surface) -> vec4<f32> { return surface.colour; }".to_string());
        let message = broken.unwrap_err();
        assert!(message.starts_with("game3d_shader:"), "the error names who raised it, got {}", message);
        assert!(message.contains("colour"), "the compiler's own message survives, got {}", message);

        let missing = shader("fn tint_only() -> f32 { return 1.0; }".to_string());
        assert!(missing.is_err(), "a module without `shade` cannot load");
    }

    #[test]
    fn shader_params_reach_the_surface() {
        let uses_params = shader("fn shade(surface: NAIL_Surface) -> vec4<f32> { return vec4<f32>(surface.color * surface.params.x, 1.0); }".to_string());
        assert!(uses_params.is_ok(), "surface.params is part of the contract: {:?}", uses_params.err());
        let dressed = draw_shader_params(draw(1, 0.0, 0.0, 0.0), 0.25, 0.5, 0.75, 1.0);
        assert_eq!((dressed.param_a, dressed.param_b, dressed.param_c, dressed.param_d), (0.25, 0.5, 0.75, 1.0));
    }

    #[test]
    fn deep_fog_swallows_a_far_cube() {
        let cube = mesh_cube().unwrap();
        let mut misty = default_environment();
        misty.fog_color = "#ff0000".to_string();
        misty.fog_near = 0.5;
        misty.fog_far = 2.0;
        let data = SceneData {
            camera: test_camera(),
            environment: misty,
            draws: vec![draw(cube, 0.0, 0.0, 10.0)],
        };
        let shapes = expand_scene(&data).unwrap();
        assert!(!shapes.is_empty());
        for shape in &shapes {
            let red = u8::from_str_radix(&shape.color[1..3], 16).unwrap();
            let green = u8::from_str_radix(&shape.color[3..5], 16).unwrap();
            let blue = u8::from_str_radix(&shape.color[5..7], 16).unwrap();
            assert!(red >= 253 && green <= 2 && blue <= 2, "past fog_far everything is the fog colour, got {}", shape.color);
        }
    }

    #[test]
    fn a_zeroed_light_direction_is_a_named_error() {
        let cube = mesh_cube().unwrap();
        let mut dark = default_environment();
        dark.light_x = 0.0;
        dark.light_y = 0.0;
        dark.light_z = 0.0;
        let data = SceneData {
            camera: test_camera(),
            environment: dark,
            draws: vec![draw(cube, 0.0, 0.0, 0.0)],
        };
        assert!(expand_scene(&data).is_err());
    }

    #[test]
    fn projecting_the_middle_of_the_view_lands_in_the_middle_of_the_screen() {
        let spot = project(test_camera(), 0.0, 0.0, 0.0);
        assert!(spot.visible);
        assert!((spot.screen_x - 400.0).abs() < 1e-6 && (spot.screen_y - 300.0).abs() < 1e-6);
        assert!((spot.depth - 3.0).abs() < 1e-6);
        let behind = project(test_camera(), 0.0, 0.0, -10.0);
        assert!(!behind.visible);
    }

    #[test]
    fn the_middle_pixel_looks_straight_ahead() {
        let pointing = ray(test_camera(), 400.0, 300.0);
        assert!((pointing.direction_x).abs() < 1e-9);
        assert!((pointing.direction_y).abs() < 1e-9);
        assert!((pointing.direction_z - 1.0).abs() < 1e-9);
    }

    #[test]
    fn picking_finds_the_nearer_cube_and_misses_the_sky() {
        let cube = mesh_cube().unwrap();
        let far_then_near = vec![draw(cube, 0.0, 0.0, 4.0), draw(cube, 0.0, 0.0, 0.0)];
        assert_eq!(pick(ray(test_camera(), 400.0, 300.0), far_then_near).unwrap(), 1, "the nearer cube wins the pick");
        let one = vec![draw(cube, 0.0, 0.0, 0.0)];
        assert!(pick(ray(test_camera(), 5.0, 5.0), one).is_err(), "the corner of the screen hits nothing");
    }

    #[test]
    fn picking_respects_scale_and_position() {
        let cube = mesh_cube().unwrap();
        // A cube pushed off centre is only under the pixels it moved to.
        let moved = vec![draw(cube, 2.0, 0.0, 0.0)];
        assert!(pick(ray(test_camera(), 400.0, 300.0), moved.clone()).is_err());
        // Scaled up five times it reaches the middle of the screen again.
        let grown = vec![draw_scaled(moved[0].clone(), 5.0, 5.0, 5.0)];
        assert_eq!(pick(ray(test_camera(), 400.0, 300.0), grown).unwrap(), 0);
    }

    #[test]
    fn mesh_vertex_floats_hands_the_card_sane_numbers() {
        let cube = mesh_cube().unwrap();
        let floats = mesh_vertex_floats(cube).unwrap();
        assert_eq!(floats.len(), 12 * 3 * 9, "twelve triangles, three corners, nine numbers each");
        for vertex in floats.chunks_exact(9) {
            let length = (vertex[3] * vertex[3] + vertex[4] * vertex[4] + vertex[5] * vertex[5]).sqrt();
            assert!((length - 1.0).abs() < 1e-4, "every normal has length one");
            assert!(vertex[6] >= 0.0 && vertex[6] <= 1.0, "colours sit in 0 to 1");
        }
        assert!(mesh_vertex_floats(888_888).is_err());
    }
}
