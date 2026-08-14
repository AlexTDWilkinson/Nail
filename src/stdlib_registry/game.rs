//! Game module stdlib registry entries.
//!
//! `game_run` is the windowed counterpart of `tui_run` and shares its whole
//! shape: the program is a state struct plus `view` and `update` functions,
//! both written in terms of the type variable `T` that `game_run`'s own
//! `initial` argument binds (see HANDLER_CALLBACKS). Everything else here is
//! a constructor returning one `GAME_Shape` value, because a frame is just an
//! array of shapes.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("game_run", StdlibFunction {
        rust_path: "std_lib::game::run".to_string(),
        crate_deps: vec![CrateDependency::Winit, CrateDependency::Softbuffer, CrateDependency::TinySkia, CrateDependency::Fontdue, CrateDependency::Tokio],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME_Config", "nail::std_lib::game"), ("GAME_Frame", "nail::std_lib::game"), ("GAME_Shape", "nail::std_lib::game"), ("GAME_Input", "nail::std_lib::game")],
        module: StdlibModule::Game,
        parameters: vec![
            StdlibParameter { name: "config".to_string(), param_type: NailDataTypeDescriptor::Struct("GAME_Config".to_string()), pass_by_reference: false },
            nail_param!(initial: T),
        ],
        return_type: nail_type!((T!e)),
        diverging: false,
        description: "Opens a window and runs a game until its view reports quit or the player closes the window, then returns the state it finished with. The program supplies two functions - view(state) returns a GAME_Frame and update(state, input) returns the next state. Input names keys as lowercase letters and digits plus Up, Down, Left, Right, Space, Enter, Esc, Shift, Ctrl, Alt, Tab and Backspace. A target_fps of 0 runs unpaced and an explicit target is honoured as written, however high - the engine imposes no ceiling of its own. In a browser, frames are paced by requestAnimationFrame regardless, so the display's refresh rate is the cap there.",
        example: "struct Pong { ball_x:f, ball_y:f }\n\nf view(state:Pong):GAME_Frame {\n    ball:GAME_Shape = game_circle(state.ball_x, state.ball_y, 8.0, `#fdd835`);\n    r GAME_Frame { shapes = [ball], background = `#101018`, quit = false };\n}\n\nf update(state:Pong, input:GAME_Input):Pong {\n    r Pong { ball_x = state.ball_x + (0.2 * input.delta_ms), ball_y = state.ball_y };\n}\n\nfinal_state:Pong = danger(game_run(GAME_Config { title = `Pong`, width = 800, height = 600, target_fps = 60, pixel_size = 1, physics_hz = 0 }, Pong { ball_x = 400.0, ball_y = 300.0 }));",
    });

    m.insert("game_rect", StdlibFunction {
        rust_path: "std_lib::game::rect".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME_Shape", "nail::std_lib::game")],
        module: StdlibModule::Game,
        parameters: vec![nail_param!(x: f), nail_param!(y: f), nail_param!(width: f), nail_param!(height: f), nail_param!(color: s)],
        return_type: NailDataTypeDescriptor::Struct("GAME_Shape".to_string()),
        diverging: false,
        description: "A filled rectangle with its top left corner at x, y. Colours everywhere in this module are strings: #rrggbb, #rrggbbaa, #rgb, or a basic name like red.",
        example: "paddle:GAME_Shape = game_rect(10.0, 250.0, 16.0, 100.0, `white`);",
    });

    m.insert("game_rect_outline", StdlibFunction {
        rust_path: "std_lib::game::rect_outline".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME_Shape", "nail::std_lib::game")],
        module: StdlibModule::Game,
        parameters: vec![nail_param!(x: f), nail_param!(y: f), nail_param!(width: f), nail_param!(height: f), nail_param!(thickness: f), nail_param!(color: s)],
        return_type: NailDataTypeDescriptor::Struct("GAME_Shape".to_string()),
        diverging: false,
        description: "Just the border of a rectangle, drawn thickness pixels wide.",
        example: "court:GAME_Shape = game_rect_outline(0.0, 0.0, 800.0, 600.0, 4.0, `gray`);",
    });

    m.insert("game_circle", StdlibFunction {
        rust_path: "std_lib::game::circle".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME_Shape", "nail::std_lib::game")],
        module: StdlibModule::Game,
        parameters: vec![nail_param!(x: f), nail_param!(y: f), nail_param!(radius: f), nail_param!(color: s)],
        return_type: NailDataTypeDescriptor::Struct("GAME_Shape".to_string()),
        diverging: false,
        description: "A filled circle centred on x, y.",
        example: "ball:GAME_Shape = game_circle(400.0, 300.0, 8.0, `#fdd835`);",
    });

    m.insert("game_line", StdlibFunction {
        rust_path: "std_lib::game::line".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME_Shape", "nail::std_lib::game")],
        module: StdlibModule::Game,
        parameters: vec![nail_param!(start_x: f), nail_param!(start_y: f), nail_param!(end_x: f), nail_param!(end_y: f), nail_param!(thickness: f), nail_param!(color: s)],
        return_type: NailDataTypeDescriptor::Struct("GAME_Shape".to_string()),
        diverging: false,
        description: "A straight line from one point to another, thickness pixels wide.",
        example: "net:GAME_Shape = game_line(400.0, 0.0, 400.0, 600.0, 2.0, `gray`);",
    });

    m.insert("game_triangle", StdlibFunction {
        rust_path: "std_lib::game::triangle".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME_Shape", "nail::std_lib::game")],
        module: StdlibModule::Game,
        parameters: vec![nail_param!(x1: f), nail_param!(y1: f), nail_param!(x2: f), nail_param!(y2: f), nail_param!(x3: f), nail_param!(y3: f), nail_param!(color: s)],
        return_type: NailDataTypeDescriptor::Struct("GAME_Shape".to_string()),
        diverging: false,
        description: "A filled triangle through three corners. The 3D module emits these, and they are just as usable straight from a program.",
        example: "sail:GAME_Shape = game_triangle(100.0, 200.0, 150.0, 100.0, 200.0, 200.0, `white`);",
    });

    m.insert("game_text", StdlibFunction {
        rust_path: "std_lib::game::text".to_string(),
        crate_deps: vec![CrateDependency::TinySkia, CrateDependency::Fontdue],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME_Shape", "nail::std_lib::game")],
        module: StdlibModule::Game,
        parameters: vec![nail_param!(content: s), nail_param!(x: f), nail_param!(y: f), nail_param!(size: f), nail_param!(color: s)],
        return_type: NailDataTypeDescriptor::Struct("GAME_Shape".to_string()),
        diverging: false,
        description: "Text whose top left corner is at x, y, drawn size pixels tall in the built-in monospace font.",
        example: "score:GAME_Shape = game_text(`3 : 2`, 360.0, 20.0, 32.0, `white`);",
    });

    m.insert("game_sprite_load", StdlibFunction {
        rust_path: "std_lib::game::sprite_load".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Game,
        parameters: vec![nail_param!(path: s)],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "Reads a PNG from disk and returns the number that names it in game_sprite from then on. Load sprites once before game_run, not inside update or view.",
        example: "ship:i = danger(game_sprite_load(`assets/ship.png`));",
    });

    m.insert("game_sprite", StdlibFunction {
        rust_path: "std_lib::game::sprite".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME_Shape", "nail::std_lib::game")],
        module: StdlibModule::Game,
        parameters: vec![nail_param!(handle: i), nail_param!(x: f), nail_param!(y: f)],
        return_type: NailDataTypeDescriptor::Struct("GAME_Shape".to_string()),
        diverging: false,
        description: "A loaded sprite drawn at its own size with its top left corner at x, y.",
        example: "ship:i = danger(game_sprite_load(`assets/ship.png`));\nplayer:GAME_Shape = game_sprite(ship, 100.0, 200.0);",
    });

    m.insert("game_sprite_scaled", StdlibFunction {
        rust_path: "std_lib::game::sprite_scaled".to_string(),
        crate_deps: vec![CrateDependency::TinySkia],
        struct_derives: vec![],
        custom_type_imports: vec![("GAME_Shape", "nail::std_lib::game")],
        module: StdlibModule::Game,
        parameters: vec![nail_param!(handle: i), nail_param!(x: f), nail_param!(y: f), nail_param!(width: f), nail_param!(height: f)],
        return_type: NailDataTypeDescriptor::Struct("GAME_Shape".to_string()),
        diverging: false,
        description: "A loaded sprite stretched to width by height at x, y.",
        example: "ship:i = danger(game_sprite_load(`assets/ship.png`));\nboss:GAME_Shape = game_sprite_scaled(ship, 300.0, 100.0, 128.0, 128.0);",
    });
}
