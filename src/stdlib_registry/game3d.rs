//! 3D module stdlib registry entries.
//!
//! The whole module is functions from meshes to arrays of ordinary 2D
//! shapes: `game3d_mesh` projects, lights and sorts, and what comes back
//! goes straight into a GAME_Frame. The camera is a plain struct the
//! program builds as a literal.

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
}
