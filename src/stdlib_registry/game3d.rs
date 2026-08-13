//! 3D module stdlib registry entries.
//!
//! Two ways in. `game3d_mesh` is a function from a mesh to an array of
//! ordinary 2D shapes, projected and sorted on the CPU. `game3d_scene`
//! wraps a whole scene - camera, environment, an array of placed draws -
//! into one shape the backend renders on the graphics card with a real
//! depth buffer, falling back to the same CPU projection when no card
//! answers. The camera is a plain struct the program builds as a literal,
//! and so is the environment for anyone not content with the default.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("game3d_mesh_load", StdlibFunction {
        rust_path: "std_lib::game3d::mesh_load".to_string(),
        crate_deps: vec![CrateDependency::Gltf, CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Game3d,
        parameters: vec![nail_param!(path: s)],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "Reads a glTF model and returns the number that names it from then on. On a real machine the path is a file on disk, in the browser build the same call fetches the path as a URL, so one program works in both worlds. Binary .glb files carry everything in one file and are the form to reach for. Textures are not read: triangles take their material's base colour, so low-poly models with coloured materials look best. Models are centred and scaled to one unit on load.",
        example: "ship:i = danger(game3d_mesh_load(`assets/ship.glb`));",
    });

    m.insert("game3d_mesh_cube", StdlibFunction {
        rust_path: "std_lib::game3d::mesh_cube".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Game3d,
        parameters: vec![],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "A generated unit cube with a different colour on each face. Something to spin before any model has been found, and a fine building block after.",
        example: "block:i = danger(game3d_mesh_cube());",
    });

    m.insert("game3d_mesh", StdlibFunction {
        rust_path: "std_lib::game3d::mesh".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Camera", "nail::std_lib::game3d"), ("GAME_Shape", "nail::std_lib::game")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "camera".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Camera".to_string()), pass_by_reference: false },
            nail_param!(handle: i),
            nail_param!(x: f),
            nail_param!(y: f),
            nail_param!(z: f),
            nail_param!(rotation_y: f),
            nail_param!(scale: f),
            nail_param!(tint: s),
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("GAME_Shape".to_string()))))),
        diverging: false,
        description: "A loaded mesh seen through a camera: placed at x, y, z, spun rotation_y radians, scaled, lit by a fixed light and depth-sorted, returned as ordinary triangle shapes for the frame. An empty tint keeps the mesh's material colours, a #rrggbb tint repaints every triangle.",
        example: "camera:GAME3D_Camera = GAME3D_Camera {\n    position_x = 0.0, position_y = 1.0, position_z = 4.0,\n    target_x = 0.0, target_y = 0.0, target_z = 0.0,\n    field_of_view = 60.0, viewport_width = 800.0, viewport_height = 600.0\n};\nship:i = danger(game3d_mesh_cube());\nangle:f = 0.5;\nspinning:a:GAME_Shape = danger(game3d_mesh(camera, ship, 0.0, 0.0, 0.0, angle, 1.0, ``));",
    });

    m.insert("game3d_line", StdlibFunction {
        rust_path: "std_lib::game3d::line".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Camera", "nail::std_lib::game3d"), ("GAME_Shape", "nail::std_lib::game")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "camera".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Camera".to_string()), pass_by_reference: false },
            nail_param!(x1: f),
            nail_param!(y1: f),
            nail_param!(z1: f),
            nail_param!(x2: f),
            nail_param!(y2: f),
            nail_param!(z2: f),
            nail_param!(thickness: f),
            nail_param!(color: s),
        ],
        return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("GAME_Shape".to_string()))),
        diverging: false,
        description: "A line between two points in the world, projected onto the frame. The array is empty when the line is behind the camera, so it can always be concatenated into the shapes.",
        example: "camera:GAME3D_Camera = GAME3D_Camera {\n    position_x = 0.0, position_y = 1.0, position_z = 4.0,\n    target_x = 0.0, target_y = 0.0, target_z = 0.0,\n    field_of_view = 60.0, viewport_width = 800.0, viewport_height = 600.0\n};\nedge:a:GAME_Shape = game3d_line(camera, -1.0, 0.0, -1.0, 1.0, 0.0, -1.0, 2.0, `gray`);",
    });

    m.insert("game3d_draw", StdlibFunction {
        rust_path: "std_lib::game3d::draw".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Draw", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![nail_param!(mesh: i), nail_param!(position_x: f), nail_param!(position_y: f), nail_param!(position_z: f)],
        return_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()),
        diverging: false,
        description: "One mesh placed in a scene at x, y, z: no spin, its own size, its own colours. The other game3d_draw functions reshape the value from there, so a program never writes the struct's seventeen fields by hand.",
        example: "ship:i = danger(game3d_mesh_cube());\nplaced:GAME3D_Draw = game3d_draw(ship, 0.0, 0.0, 0.0);",
    });

    m.insert("game3d_draw_rotated", StdlibFunction {
        rust_path: "std_lib::game3d::draw_rotated".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Draw", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "draw".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()), pass_by_reference: false },
            nail_param!(rotation_x: f),
            nail_param!(rotation_y: f),
            nail_param!(rotation_z: f),
        ],
        return_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()),
        diverging: false,
        description: "The same draw spun to the given angles in radians, applied about x, then y, then z. Spinning about y alone is the usual turntable, x pitches forward and back, z rolls.",
        example: "ship:i = danger(game3d_mesh_cube());\nspun:GAME3D_Draw = game3d_draw_rotated(game3d_draw(ship, 0.0, 0.0, 0.0), 0.0, 0.8, 0.0);",
    });

    m.insert("game3d_draw_scaled", StdlibFunction {
        rust_path: "std_lib::game3d::draw_scaled".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Draw", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "draw".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()), pass_by_reference: false },
            nail_param!(scale_x: f),
            nail_param!(scale_y: f),
            nail_param!(scale_z: f),
        ],
        return_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()),
        diverging: false,
        description: "The same draw stretched along each axis. The same number three times scales evenly, different numbers squash and stretch, and a unit ground mesh scaled by forty is a floor.",
        example: "floor_mesh:i = danger(game3d_mesh_ground(`#3a5f3a`, `#2c4a2c`, 8));\nfloor:GAME3D_Draw = game3d_draw_scaled(game3d_draw(floor_mesh, 0.0, -0.5, 0.0), 40.0, 1.0, 40.0);",
    });

    m.insert("game3d_draw_tinted", StdlibFunction {
        rust_path: "std_lib::game3d::draw_tinted".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Draw", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "draw".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()), pass_by_reference: false },
            nail_param!(tint: s),
        ],
        return_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()),
        diverging: false,
        description: "The same draw repainted: every triangle takes this #rrggbb colour instead of the mesh's own, which is how one mesh serves as both the red team and the blue team. An empty string goes back to the mesh's colours.",
        example: "ball:i = danger(game3d_mesh_sphere(`white`, 16));\nenemy:GAME3D_Draw = game3d_draw_tinted(game3d_draw(ball, 2.0, 0.0, 1.0), `#e53935`);",
    });

    m.insert("game3d_draw_glowing", StdlibFunction {
        rust_path: "std_lib::game3d::draw_glowing".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Draw", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "draw".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()), pass_by_reference: false },
            nail_param!(glow: f),
        ],
        return_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()),
        diverging: false,
        description: "The same draw lit from within, 0 to 1. At 0 the scene's light shades it like everything else, at 1 it shows its full colour whatever the light does - a sun, a lamp, lava. Glow burns through fog in the same proportion, so at 1 a distant sun stays blazing instead of greying into the haze, the way a light does in real fog.",
        example: "sun:i = danger(game3d_mesh_sphere(`#fdd835`, 20));\nblazing:GAME3D_Draw = game3d_draw_glowing(game3d_draw(sun, 0.0, 5.0, 0.0), 1.0);",
    });

    m.insert("game3d_shader", StdlibFunction {
        rust_path: "std_lib::game3d::shader".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Shader", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![nail_param!(source: s)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("GAME3D_Shader".to_string()))),
        diverging: false,
        description: "Loads a surface shader from WGSL source, for `game3d_draw_shaded`. The source defines one function, `fn shade(surface: NAIL_Surface) -> vec4<f32>`, plus any helpers it wants. The surface carries `color` (the mesh colour with any tint mixed in), `normal`, `world_position`, `toward_eye`, the scene's `light` direction, `light_color` and `ambient`, `time` in seconds for animation, the draw's `glow`, and `params`, the draw's own four numbers from `game3d_draw_shader_params`. The engine keeps owning placement, instancing and fog - the shader only turns a surface into a colour. The WGSL is compiled here, once, so a typo fails at load with the compiler's message instead of at first draw. The CPU fallback renderer cannot run WGSL and shades such draws the builtin way.",
        example: "flame_wgsl:s = wgsl`fn shade(surface: NAIL_Surface) -> vec4<f32> {\n    let core = pow(max(dot(surface.normal, surface.toward_eye), 0.0), 1.5);\n    return vec4<f32>(mix(surface.color, vec3<f32>(1.0, 0.97, 0.88), core), 1.0);\n}`;\nflame:GAME3D_Shader = danger(game3d_shader(flame_wgsl));",
    });

    m.insert("game3d_draw_shaded", StdlibFunction {
        rust_path: "std_lib::game3d::draw_shaded".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Draw", "nail::std_lib::game3d"), ("GAME3D_Shader", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "draw".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()), pass_by_reference: false },
            StdlibParameter { name: "shader".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Shader".to_string()), pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()),
        diverging: false,
        description: "The same draw painted by a shader loaded with `game3d_shader` instead of the builtin lighting. Draws sharing a mesh and a shader still land in one instanced draw call, so a whole ocean or a field of flames stays one batch.",
        example: "flat:i = danger(game3d_mesh_plane(`#1a3a5c`));\nsteady_wgsl:s = wgsl`fn shade(surface: NAIL_Surface) -> vec4<f32> { return vec4<f32>(surface.color, 1.0); }`;\nunlit:GAME3D_Shader = danger(game3d_shader(steady_wgsl));\nsea:GAME3D_Draw = game3d_draw_shaded(game3d_draw(flat, 0.0, -0.4, 0.0), unlit);",
    });

    m.insert("game3d_draw_shader_params", StdlibFunction {
        rust_path: "std_lib::game3d::draw_shader_params".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Draw", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "draw".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()), pass_by_reference: false },
            nail_param!(param_a: f),
            nail_param!(param_b: f),
            nail_param!(param_c: f),
            nail_param!(param_d: f),
        ],
        return_type: NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()),
        diverging: false,
        description: "Four numbers handed to the draw's custom shader as `surface.params`, so one shader serves many draws: the same water rougher here and calmer there, the same fire hotter and colder. What each number means is the shader's own business. Draws sharing a mesh and a shader still batch whatever their params, the numbers ride the instance.",
        example: "flat:i = danger(game3d_mesh_plane(`#1a3a5c`));\nripple_wgsl:s = wgsl`fn shade(surface: NAIL_Surface) -> vec4<f32> {\n    let lift = sin(surface.world_position.x * 2.0 + surface.time * surface.params.x);\n    return vec4<f32>(surface.color * (0.6 + 0.4 * lift), 1.0);\n}`;\nripple:GAME3D_Shader = danger(game3d_shader(ripple_wgsl));\ncalm:GAME3D_Draw = game3d_draw_shader_params(game3d_draw_shaded(game3d_draw(flat, 0.0, 0.0, 0.0), ripple), 0.4, 0.0, 0.0, 0.0);",
    });

    m.insert("game3d_default_environment", StdlibFunction {
        rust_path: "std_lib::game3d::default_environment".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Environment", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Struct("GAME3D_Environment".to_string()),
        diverging: false,
        description: "The environment most scenes want: an overhead sun a little to the side, white light, a sensible ambient floor, no fog. Nail structs have no default field values, so this saves spelling all eight out - a custom sky writes the GAME3D_Environment literal instead, the way a camera is written.",
        example: "air:GAME3D_Environment = game3d_default_environment();",
    });

    m.insert("game3d_scene", StdlibFunction {
        rust_path: "std_lib::game3d::scene".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![
            ("GAME3D_Camera", "nail::std_lib::game3d"),
            ("GAME3D_Environment", "nail::std_lib::game3d"),
            ("GAME3D_Draw", "nail::std_lib::game3d"),
            ("GAME_Shape", "nail::std_lib::game"),
        ],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "camera".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Camera".to_string()), pass_by_reference: false },
            StdlibParameter { name: "environment".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Environment".to_string()), pass_by_reference: false },
            StdlibParameter { name: "draws".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()))), pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::Struct("GAME_Shape".to_string()),
        diverging: false,
        description: "A whole 3D scene as one shape for the frame: these meshes, placed like this, seen through this camera, lit like that. On a machine with a graphics card it renders there, depth buffered and instanced, and shapes before it in the frame stay under it while shapes after it draw over it, which is exactly where a HUD goes. Without a card it becomes depth-sorted triangles on the CPU, same picture, fewer frames. Build the scene fresh in view each frame - the frame draws it once and it is gone.",
        example: "camera:GAME3D_Camera = GAME3D_Camera {\n    position_x = 0.0, position_y = 2.0, position_z = 5.0,\n    target_x = 0.0, target_y = 0.0, target_z = 0.0,\n    field_of_view = 60.0, viewport_width = 800.0, viewport_height = 600.0\n};\nblock:i = danger(game3d_mesh_cube());\nworld:GAME_Shape = game3d_scene(camera, game3d_default_environment(), [game3d_draw(block, 0.0, 0.0, 0.0)]);",
    });

    m.insert("game3d_mesh_sphere", StdlibFunction {
        rust_path: "std_lib::game3d::mesh_sphere".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Game3d,
        parameters: vec![nail_param!(color: s), nail_param!(bands: i)],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "A generated sphere of one colour, half a unit across so it fills the same box a cube does. `bands` is how many horizontal slices build it: 3 is a gem, 24 is smooth, and anything outside 3 to 48 is clamped.",
        example: "ball:i = danger(game3d_mesh_sphere(`#1e88e5`, 16));",
    });

    m.insert("game3d_mesh_plane", StdlibFunction {
        rust_path: "std_lib::game3d::mesh_plane".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Game3d,
        parameters: vec![nail_param!(color: s)],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "A flat unit square of one colour lying in the ground plane, visible from above and below. Scaled up it is a floor, tilted it is a wall or a ramp.",
        example: "slab:i = danger(game3d_mesh_plane(`gray`));",
    });

    m.insert("game3d_mesh_ground", StdlibFunction {
        rust_path: "std_lib::game3d::mesh_ground".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Game3d,
        parameters: vec![nail_param!(color_a: s), nail_param!(color_b: s), nail_param!(squares: i)],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "A unit checkerboard, `squares` cells along each side alternating between the two colours, visible from above and below. Scale a draw of it up a hundred times and there is a floor with visible perspective for free, which is the fastest way to make a 3D scene read as a place.",
        example: "floor_mesh:i = danger(game3d_mesh_ground(`#3a5f3a`, `#2c4a2c`, 8));",
    });

    m.insert("game3d_mesh_cylinder", StdlibFunction {
        rust_path: "std_lib::game3d::mesh_cylinder".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Game3d,
        parameters: vec![nail_param!(color: s), nail_param!(sides: i)],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "A generated cylinder of one colour standing on the y axis, one unit tall and half a unit across, with caps. `sides` is how many flat faces stand in for the curve, 3 to 64. Pillars, tree trunks, wheels lying down.",
        example: "pillar:i = danger(game3d_mesh_cylinder(`#9e9e9e`, 24));",
    });

    m.insert("game3d_mesh_from_triangles", StdlibFunction {
        rust_path: "std_lib::game3d::mesh_from_triangles".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Game3d,
        parameters: vec![nail_param!(positions: [f]), nail_param!(colors: [s])],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "A mesh built straight from numbers: nine per triangle, three corners of x, y and z, wound counter-clockwise seen from outside, and exactly one colour per triangle - array_repeat turns one colour into a whole mesh's worth. Corners are kept exactly as given, no centring and no scaling, because whoever computes terrain knows where it goes. This is the raw material of anything procedural: heightfields, voxels, whole worlds from arithmetic.",
        example: "positions:a:f = [0.0, 0.0, 0.0, 0.5, 1.0, 0.0, 1.0, 0.0, 0.0];\ncolors:a:s = array_repeat(`#e53935`, 1);\nfacet:i = danger(game3d_mesh_from_triangles(positions, colors));",
    });

    m.insert("game3d_project", StdlibFunction {
        rust_path: "std_lib::game3d::project".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Camera", "nail::std_lib::game3d"), ("GAME3D_ScreenPoint", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "camera".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Camera".to_string()), pass_by_reference: false },
            nail_param!(x: f),
            nail_param!(y: f),
            nail_param!(z: f),
        ],
        return_type: NailDataTypeDescriptor::Struct("GAME3D_ScreenPoint".to_string()),
        diverging: false,
        description: "Where a world point lands on the screen, how far in front of the camera it sits, and whether it is really there to see. This is how a name tag, a health bar or a damage number drawn with 2D shapes follows something that lives in the 3D world.",
        example: "camera:GAME3D_Camera = GAME3D_Camera {\n    position_x = 0.0, position_y = 2.0, position_z = 5.0,\n    target_x = 0.0, target_y = 0.0, target_z = 0.0,\n    field_of_view = 60.0, viewport_width = 800.0, viewport_height = 600.0\n};\nlabel_spot:GAME3D_ScreenPoint = game3d_project(camera, 0.0, 1.2, 0.0);",
    });

    m.insert("game3d_ray", StdlibFunction {
        rust_path: "std_lib::game3d::ray".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Camera", "nail::std_lib::game3d"), ("GAME3D_Ray", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "camera".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Camera".to_string()), pass_by_reference: false },
            nail_param!(screen_x: f),
            nail_param!(screen_y: f),
        ],
        return_type: NailDataTypeDescriptor::Struct("GAME3D_Ray".to_string()),
        diverging: false,
        description: "The ray under a screen pixel: the camera's own position, and the direction of length one that pixel looks along. Feed the mouse to it and hand the result to game3d_pick, or intersect it with your own arithmetic for aiming, building placement, or anything else that starts at the screen and means the world.",
        example: "camera:GAME3D_Camera = GAME3D_Camera {\n    position_x = 0.0, position_y = 2.0, position_z = 5.0,\n    target_x = 0.0, target_y = 0.0, target_z = 0.0,\n    field_of_view = 60.0, viewport_width = 800.0, viewport_height = 600.0\n};\nunder_mouse:GAME3D_Ray = game3d_ray(camera, 400.0, 300.0);",
    });

    m.insert("game3d_pick", StdlibFunction {
        rust_path: "std_lib::game3d::pick".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME3D_Ray", "nail::std_lib::game3d"), ("GAME3D_Draw", "nail::std_lib::game3d")],
        module: StdlibModule::Game3d,
        parameters: vec![
            StdlibParameter { name: "ray".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME3D_Ray".to_string()), pass_by_reference: false },
            StdlibParameter { name: "draws".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("GAME3D_Draw".to_string()))), pass_by_reference: false },
        ],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "Which draw a ray hits: the index into the array of the nearest mesh whose bounding box the ray passes through, or an error when it touches none of them. With the ray from game3d_ray under the mouse, this is clicking on a unit, and safe() turns the miss into whatever a miss means to the game.",
        example: "camera:GAME3D_Camera = GAME3D_Camera {\n    position_x = 0.0, position_y = 2.0, position_z = 5.0,\n    target_x = 0.0, target_y = 0.0, target_z = 0.0,\n    field_of_view = 60.0, viewport_width = 800.0, viewport_height = 600.0\n};\nblock:i = danger(game3d_mesh_cube());\nunits:a:GAME3D_Draw = [game3d_draw(block, 0.0, 0.0, 0.0)];\nclicked:i = danger(game3d_pick(game3d_ray(camera, 400.0, 300.0), units));",
    });
}
