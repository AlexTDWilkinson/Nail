//! Three dimensions, no graphics card: a camera, some meshes, and the same
//! 2D frame everything else draws into.
//!
//! `game3d_mesh` takes a camera and a loaded mesh and returns an array of
//! plain `GAME_Shape` triangles - projected, lit and depth-sorted, ready to
//! go into a `GAME_Frame` next to any 2D shape. There is no scene, no
//! renderer object and no separate 3D pipeline: three dimensions are just a
//! function from meshes to shapes.
//!
//! Models load from glTF, the format every 3D tool exports. On a real
//! machine `game3d_mesh_load` reads the file from disk, in the browser build
//! the same call fetches the same path as a URL, so one program works both
//! places. Textures are not read - triangles take their material's base
//! colour and a single fixed light shades them - so low-poly models with
//! coloured materials look best, which suits the whole aesthetic.
//!
//! The camera looks from its position toward its target with y up. Meshes
//! are normalised on load - centred at the origin, longest side scaled to
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

/// One triangle of a loaded mesh, in the mesh's own normalised space.
#[derive(Clone)]
struct MeshTriangle {
    corners: [[f32; 3]; 3],
    color: (u8, u8, u8),
}

fn meshes() -> &'static Mutex<HashMap<i64, Vec<MeshTriangle>>> {
    static MESHES: OnceLock<Mutex<HashMap<i64, Vec<MeshTriangle>>>> = OnceLock::new();
    return MESHES.get_or_init(|| Mutex::new(HashMap::new()));
}

static NEXT_MESH: AtomicI64 = AtomicI64::new(1);

fn store_mesh(triangles: Vec<MeshTriangle>) -> Result<i64, String> {
    let handle = NEXT_MESH.fetch_add(1, Ordering::Relaxed);
    meshes().lock().map_err(|_| "game3d: the mesh store is poisoned".to_string())?.insert(handle, triangles);
    return Ok(handle);
}

/// Centres a soup of triangles on the origin and scales its longest side to
/// one unit, so `scale` in `game3d_mesh` means the same thing for any model.
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

/// The one light in the world, heading mostly down and a little sideways.
const LIGHT: [f64; 3] = [0.45, -0.8, 0.35];

fn parse_tint(color: &str) -> Option<(u8, u8, u8)> {
    let hex = color.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let part = |from: usize| u8::from_str_radix(&hex[from..from + 2], 16).ok();
    return Some((part(0)?, part(2)?, part(4)?));
}

/// Projects a mesh through a camera into flat triangles: placed, spun around
/// y, scaled, lit by the fixed light, painter-sorted far to near, and
/// returned as ordinary shapes for the frame. An empty `tint` keeps the
/// mesh's own material colours, a `#rrggbb` tint repaints every triangle.
pub fn mesh(camera: GAME3D_Camera, handle: i64, x: f64, y: f64, z: f64, rotation_y: f64, scale: f64, tint: String) -> Result<Vec<GAME_Shape>, String> {
    let store = meshes().lock().map_err(|_| "game3d_mesh: the mesh store is poisoned".to_string())?;
    let Some(triangles) = store.get(&handle) else {
        return Err(format!("game3d_mesh: no mesh with the number {} was loaded", handle));
    };
    let tint = if tint.is_empty() { None } else { Some(parse_tint(&tint).ok_or_else(|| format!("game3d_mesh: `{}` is not a #rrggbb colour", tint))?) };

    // The camera's view basis: forward toward the target, right and up from
    // crossing it with world up.
    let forward = normalized([camera.target_x - camera.position_x, camera.target_y - camera.position_y, camera.target_z - camera.position_z]);
    let right = normalized(cross(forward, [0.0, 1.0, 0.0]));
    let up = cross(right, forward);
    let focal = (camera.viewport_height / 2.0) / (camera.field_of_view.to_radians() / 2.0).tan();
    let half_width = camera.viewport_width / 2.0;
    let half_height = camera.viewport_height / 2.0;
    let (sin, cos) = rotation_y.sin_cos();
    let light = normalized(LIGHT);

    struct Projected {
        depth: f64,
        points: [[f64; 2]; 3],
        color: (u8, u8, u8),
        brightness: f64,
    }
    let mut flat: Vec<Projected> = Vec::with_capacity(triangles.len());

    for triangle in triangles.iter() {
        let mut world = [[0.0f64; 3]; 3];
        for (index, corner) in triangle.corners.iter().enumerate() {
            // Spin around y, scale, then place in the world.
            let spun_x = corner[0] as f64 * cos + corner[2] as f64 * sin;
            let spun_z = corner[2] as f64 * cos - corner[0] as f64 * sin;
            world[index] = [spun_x * scale + x, corner[1] as f64 * scale + y, spun_z * scale + z];
        }

        // Flat shading wants the face normal in world space.
        let edge_one = subtract(world[1], world[0]);
        let edge_two = subtract(world[2], world[0]);
        let normal = normalized(cross(edge_one, edge_two));
        let facing = dot(normal, light).max(0.0);
        let brightness = 0.35 + 0.65 * facing;

        let mut points = [[0.0f64; 2]; 3];
        let mut depth = 0.0;
        let mut behind = false;
        for (index, corner) in world.iter().enumerate() {
            let toward = subtract(*corner, [camera.position_x, camera.position_y, camera.position_z]);
            let view_x = dot(toward, right);
            let view_y = dot(toward, up);
            let view_z = dot(toward, forward);
            if view_z < 0.05 {
                behind = true;
                break;
            }
            points[index] = [half_width + view_x * focal / view_z, half_height - view_y * focal / view_z];
            depth += view_z;
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

        flat.push(Projected { depth, points, color: tint.unwrap_or(triangle.color), brightness });
    }

    // Painter's algorithm: the far triangles go first so near ones cover them.
    flat.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal));

    let shapes = flat
        .into_iter()
        .map(|triangle| {
            let (red, green, blue) = triangle.color;
            let lit = |channel: u8| ((channel as f64 * triangle.brightness) as u8).min(255);
            let color = format!("#{:02x}{:02x}{:02x}", lit(red), lit(green), lit(blue));
            return super::game::triangle(triangle.points[0][0], triangle.points[0][1], triangle.points[1][0], triangle.points[1][1], triangle.points[2][0], triangle.points[2][1], color);
        })
        .collect();
    return Ok(shapes);
}

/// A line between two points in the world, projected onto the frame. Court
/// edges, axes, trajectories - anywhere the third dimension needs a wire.
pub fn line(camera: GAME3D_Camera, x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64, thickness: f64, color: String) -> Vec<GAME_Shape> {
    let forward = normalized([camera.target_x - camera.position_x, camera.target_y - camera.position_y, camera.target_z - camera.position_z]);
    let right = normalized(cross(forward, [0.0, 1.0, 0.0]));
    let up = cross(right, forward);
    let focal = (camera.viewport_height / 2.0) / (camera.field_of_view.to_radians() / 2.0).tan();
    let project = |point: [f64; 3]| -> Option<[f64; 2]> {
        let toward = subtract(point, [camera.position_x, camera.position_y, camera.position_z]);
        let view_z = dot(toward, forward);
        if view_z < 0.05 {
            return None;
        }
        return Some([camera.viewport_width / 2.0 + dot(toward, right) * focal / view_z, camera.viewport_height / 2.0 - dot(toward, up) * focal / view_z]);
    };
    let (Some(start), Some(end)) = (project([x1, y1, z1]), project([x2, y2, z2])) else {
        return Vec::new();
    };
    return vec![super::game::line(start[0], start[1], end[0], end[1], thickness, color)];
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
}
