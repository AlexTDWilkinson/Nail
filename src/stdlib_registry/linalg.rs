//! Linear algebra module stdlib registry entries.
//!
//! Every function here names one of the module's own struct types, which the
//! shared `simple_fns!` macro has no spelling for - it only knows Nail's
//! primitive type letters. Rather than write out sixty full `StdlibFunction`
//! literals, this file has its own pair of macros: `linalg_type!` adds one arm
//! for the struct types on top of the letters, and `linalg_fns!` is the same
//! shape as `simple_fns!` using it.

use super::*;

/// The types a linalg signature can name: Nail's primitive letters, arrays and
/// results as usual, plus a bare `Vec2`, `Vec3` or `Mat3` for this module's own
/// structs, which stand for `LINALG_Vec2` and so on.
macro_rules! linalg_type {
    (f) => { NailDataTypeDescriptor::Float };
    (i) => { NailDataTypeDescriptor::Int };
    (b) => { NailDataTypeDescriptor::Boolean };
    ([ $($inner:tt)+ ]) => { NailDataTypeDescriptor::Array(Box::new(linalg_type!($($inner)+))) };
    (($inner:tt !e)) => { NailDataTypeDescriptor::Result(Box::new(linalg_type!($inner))) };
    ($struct_name:ident) => { NailDataTypeDescriptor::Struct(concat!("LINALG_", stringify!($struct_name)).to_string()) };
}

macro_rules! linalg_fns {
    ($m:ident:
        $( $name:literal => $path:literal, ($($pname:ident: $ptype:tt),*) -> $ret:tt, $desc:literal, $example:literal; )*
    ) => {
        $(
            $m.insert($name, StdlibFunction {
                rust_path: $path.to_string(),
                crate_deps: vec![],
                struct_derives: vec![],
                custom_type_imports: vec![],
                module: StdlibModule::Linalg,
                parameters: vec![ $( StdlibParameter { name: stringify!($pname).to_string(), param_type: linalg_type!($ptype), pass_by_reference: false } ),* ],
                return_type: linalg_type!($ret),
                diverging: false,
                description: $desc,
                example: $example,
            });
        )*
    };
}

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    linalg_fns! { m:
        // Vec2
        "linalg_vec2" => "std_lib::linalg::vec2", (x: f, y: f) -> Vec2,
            "A point or direction in the plane.",
            "position:LINALG_Vec2 = linalg_vec2(3.0, 4.0);";
        "linalg_vec2_zero" => "std_lib::linalg::vec2_zero", () -> Vec2,
            "The origin: both components zero.",
            "start:LINALG_Vec2 = linalg_vec2_zero();";
        "linalg_vec2_add" => "std_lib::linalg::vec2_add", (first: Vec2, second: Vec2) -> Vec2,
            "Adds two vectors component by component - a position moved by a direction.",
            "position:LINALG_Vec2 = linalg_vec2(3.0, 4.0);\noffset:LINALG_Vec2 = linalg_vec2(1.0, 0.0);\nmoved:LINALG_Vec2 = linalg_vec2_add(position, offset);";
        "linalg_vec2_subtract" => "std_lib::linalg::vec2_subtract", (first: Vec2, second: Vec2) -> Vec2,
            "Subtracts the second vector from the first - the direction from second to first.",
            "target:LINALG_Vec2 = linalg_vec2(10.0, 12.0);\nposition:LINALG_Vec2 = linalg_vec2(3.0, 4.0);\ntoward:LINALG_Vec2 = linalg_vec2_subtract(target, position);";
        "linalg_vec2_scale" => "std_lib::linalg::vec2_scale", (vector: Vec2, factor: f) -> Vec2,
            "Multiplies both components by one number, which lengthens or shortens the vector without turning it.",
            "direction:LINALG_Vec2 = linalg_vec2(1.0, 0.0);\ndoubled:LINALG_Vec2 = linalg_vec2_scale(direction, 2.0);";
        "linalg_vec2_multiply" => "std_lib::linalg::vec2_multiply", (first: Vec2, second: Vec2) -> Vec2,
            "Multiplies two vectors component by component, which is a separate scale for each axis rather than any kind of vector product.",
            "size:LINALG_Vec2 = linalg_vec2(800.0, 600.0);\nfactors:LINALG_Vec2 = linalg_vec2(2.0, 0.5);\nstretched:LINALG_Vec2 = linalg_vec2_multiply(size, factors);";
        "linalg_vec2_divide" => "std_lib::linalg::vec2_divide", (first: Vec2, second: Vec2) -> (Vec2!e),
            "Divides component by component. A zero component in the second vector is an error rather than an infinity.",
            "size:LINALG_Vec2 = linalg_vec2(800.0, 600.0);\ncell:LINALG_Vec2 = linalg_vec2(16.0, 16.0);\nratio:LINALG_Vec2 = danger(linalg_vec2_divide(size, cell));";
        "linalg_vec2_negate" => "std_lib::linalg::vec2_negate", (vector: Vec2) -> Vec2,
            "The vector of the same length pointing the opposite way.",
            "direction:LINALG_Vec2 = linalg_vec2(1.0, 0.0);\nback:LINALG_Vec2 = linalg_vec2_negate(direction);";
        "linalg_vec2_dot" => "std_lib::linalg::vec2_dot", (first: Vec2, second: Vec2) -> f,
            "The dot product: positive when the vectors point the same way, zero when they are at right angles, negative when they oppose.",
            "heading:LINALG_Vec2 = linalg_vec2(1.0, 0.0);\ntoward_target:LINALG_Vec2 = linalg_vec2(0.0, 1.0);\nalignment:f = linalg_vec2_dot(heading, toward_target);";
        "linalg_vec2_length" => "std_lib::linalg::vec2_length", (vector: Vec2) -> f,
            "How long the vector is.",
            "velocity:LINALG_Vec2 = linalg_vec2(3.0, 4.0);\nspeed:f = linalg_vec2_length(velocity);";
        "linalg_vec2_length_squared" => "std_lib::linalg::vec2_length_squared", (vector: Vec2) -> f,
            "The length multiplied by itself, without the square root. Comparing two of these answers which vector is longer for less work.",
            "velocity:LINALG_Vec2 = linalg_vec2(3.0, 4.0);\nrough:f = linalg_vec2_length_squared(velocity);";
        "linalg_vec2_distance" => "std_lib::linalg::vec2_distance", (first: Vec2, second: Vec2) -> f,
            "How far apart two points are.",
            "here:LINALG_Vec2 = linalg_vec2(0.0, 0.0);\nthere:LINALG_Vec2 = linalg_vec2(3.0, 4.0);\ngap:f = linalg_vec2_distance(here, there);";
        "linalg_vec2_normalize" => "std_lib::linalg::vec2_normalize", (vector: Vec2) -> (Vec2!e),
            "The vector of length one pointing the same way. A vector of zero length points in no direction, so that is an error.",
            "velocity:LINALG_Vec2 = linalg_vec2(3.0, 4.0);\nheading:LINALG_Vec2 = danger(linalg_vec2_normalize(velocity));";
        "linalg_vec2_perpendicular" => "std_lib::linalg::vec2_perpendicular", (vector: Vec2) -> Vec2,
            "The vector at a right angle to this one, turned a quarter turn the way the coordinates grow.",
            "along_edge:LINALG_Vec2 = linalg_vec2(1.0, 0.0);\nnormal:LINALG_Vec2 = linalg_vec2_perpendicular(along_edge);";
        "linalg_vec2_rotate" => "std_lib::linalg::vec2_rotate", (vector: Vec2, radians: f) -> Vec2,
            "Turns a vector about the origin by an angle in radians.",
            "direction:LINALG_Vec2 = linalg_vec2(1.0, 0.0);\nturned:LINALG_Vec2 = linalg_vec2_rotate(direction, math_to_radians(90.0));";
        "linalg_vec2_lerp" => "std_lib::linalg::vec2_lerp", (start: Vec2, end: Vec2, t: f) -> Vec2,
            "The point part of the way from start to end, with t clamped to 0.0..1.0.",
            "start:LINALG_Vec2 = linalg_vec2(0.0, 0.0);\nfinish:LINALG_Vec2 = linalg_vec2(10.0, 4.0);\nhalfway:LINALG_Vec2 = linalg_vec2_lerp(start, finish, 0.5);";
        "linalg_vec2_min" => "std_lib::linalg::vec2_min", (first: Vec2, second: Vec2) -> Vec2,
            "The smaller of each component - one corner of the box holding both points.",
            "first_corner:LINALG_Vec2 = linalg_vec2(4.0, 9.0);\nsecond_corner:LINALG_Vec2 = linalg_vec2(1.0, 12.0);\ntop_left:LINALG_Vec2 = linalg_vec2_min(first_corner, second_corner);";
        "linalg_vec2_max" => "std_lib::linalg::vec2_max", (first: Vec2, second: Vec2) -> Vec2,
            "The larger of each component - the opposite corner of the box holding both points.",
            "first_corner:LINALG_Vec2 = linalg_vec2(4.0, 9.0);\nsecond_corner:LINALG_Vec2 = linalg_vec2(1.0, 12.0);\nbottom_right:LINALG_Vec2 = linalg_vec2_max(first_corner, second_corner);";
        "linalg_vec2_clamp" => "std_lib::linalg::vec2_clamp", (vector: Vec2, low: Vec2, high: Vec2) -> Vec2,
            "Keeps a point inside a box, one component at a time.",
            "position:LINALG_Vec2 = linalg_vec2(3.0, 4.0);\ntop_left:LINALG_Vec2 = linalg_vec2(0.0, 0.0);\nbottom_right:LINALG_Vec2 = linalg_vec2(100.0, 100.0);\ninside:LINALG_Vec2 = linalg_vec2_clamp(position, top_left, bottom_right);";
        "linalg_vec2_reflect" => "std_lib::linalg::vec2_reflect", (vector: Vec2, normal: Vec2) -> Vec2,
            "Bounces a vector off a surface facing the given direction - where a ball goes when it hits a wall. The normal should have length one.",
            "velocity:LINALG_Vec2 = linalg_vec2(3.0, 4.0);\nwall_normal:LINALG_Vec2 = linalg_vec2(0.0, 1.0);\nbounced:LINALG_Vec2 = linalg_vec2_reflect(velocity, wall_normal);";
        "linalg_vec2_angle_between" => "std_lib::linalg::vec2_angle_between", (first: Vec2, second: Vec2) -> (f!e),
            "The angle between two vectors in radians, from 0.0 to pi. A vector of zero length has no angle, so that is an error.",
            "heading:LINALG_Vec2 = linalg_vec2(1.0, 0.0);\ntoward_target:LINALG_Vec2 = linalg_vec2(0.0, 1.0);\nangle:f = danger(linalg_vec2_angle_between(heading, toward_target));";
        "linalg_vec2_equals" => "std_lib::linalg::vec2_equals", (first: Vec2, second: Vec2, tolerance: f) -> b,
            "True when two vectors match to within the given tolerance, which is how floats have to be compared once any arithmetic has happened to them.",
            "computed:LINALG_Vec2 = linalg_vec2(1.0, 2.0);\nexpected:LINALG_Vec2 = linalg_vec2(1.0, 2.0);\nsame:b = linalg_vec2_equals(computed, expected, 0.0001);";
        "linalg_vec2_to_array" => "std_lib::linalg::vec2_to_array", (vector: Vec2) -> [f],
            "The two components as an array, in x, y order - the form draw_polyline and draw_polygon take.",
            "position:LINALG_Vec2 = linalg_vec2(3.0, 4.0);\npair:a:f = linalg_vec2_to_array(position);";
        "linalg_vec2_from_array" => "std_lib::linalg::vec2_from_array", (values: [f]) -> (Vec2!e),
            "A vector from an array of exactly two numbers.",
            "pair:a:f = [3.0, 4.0];\nposition:LINALG_Vec2 = danger(linalg_vec2_from_array(pair));";

        // Vec3
        "linalg_vec3" => "std_lib::linalg::vec3", (x: f, y: f, z: f) -> Vec3,
            "A point or direction in space.",
            "position:LINALG_Vec3 = linalg_vec3(1.0, 2.0, 3.0);";
        "linalg_vec3_zero" => "std_lib::linalg::vec3_zero", () -> Vec3,
            "The origin: all three components zero.",
            "start:LINALG_Vec3 = linalg_vec3_zero();";
        "linalg_vec3_add" => "std_lib::linalg::vec3_add", (first: Vec3, second: Vec3) -> Vec3,
            "Adds two vectors component by component - a position moved by a direction.",
            "position:LINALG_Vec3 = linalg_vec3(1.0, 2.0, 3.0);\noffset:LINALG_Vec3 = linalg_vec3(0.0, 1.0, 0.0);\nmoved:LINALG_Vec3 = linalg_vec3_add(position, offset);";
        "linalg_vec3_subtract" => "std_lib::linalg::vec3_subtract", (first: Vec3, second: Vec3) -> Vec3,
            "Subtracts the second vector from the first - the direction from second to first.",
            "target:LINALG_Vec3 = linalg_vec3(4.0, 6.0, 8.0);\nposition:LINALG_Vec3 = linalg_vec3(1.0, 2.0, 3.0);\ntoward:LINALG_Vec3 = linalg_vec3_subtract(target, position);";
        "linalg_vec3_scale" => "std_lib::linalg::vec3_scale", (vector: Vec3, factor: f) -> Vec3,
            "Multiplies every component by one number, which lengthens or shortens the vector without turning it.",
            "direction:LINALG_Vec3 = linalg_vec3(1.0, 0.0, 0.0);\ndoubled:LINALG_Vec3 = linalg_vec3_scale(direction, 2.0);";
        "linalg_vec3_multiply" => "std_lib::linalg::vec3_multiply", (first: Vec3, second: Vec3) -> Vec3,
            "Multiplies two vectors component by component, which is a separate scale for each axis rather than any kind of vector product.",
            "size:LINALG_Vec3 = linalg_vec3(64.0, 32.0, 16.0);\nfactors:LINALG_Vec3 = linalg_vec3(2.0, 2.0, 1.0);\nstretched:LINALG_Vec3 = linalg_vec3_multiply(size, factors);";
        "linalg_vec3_divide" => "std_lib::linalg::vec3_divide", (first: Vec3, second: Vec3) -> (Vec3!e),
            "Divides component by component. A zero component in the second vector is an error rather than an infinity.",
            "size:LINALG_Vec3 = linalg_vec3(64.0, 32.0, 16.0);\ncell:LINALG_Vec3 = linalg_vec3(8.0, 8.0, 8.0);\nratio:LINALG_Vec3 = danger(linalg_vec3_divide(size, cell));";
        "linalg_vec3_negate" => "std_lib::linalg::vec3_negate", (vector: Vec3) -> Vec3,
            "The vector of the same length pointing the opposite way.",
            "direction:LINALG_Vec3 = linalg_vec3(1.0, 0.0, 0.0);\nback:LINALG_Vec3 = linalg_vec3_negate(direction);";
        "linalg_vec3_dot" => "std_lib::linalg::vec3_dot", (first: Vec3, second: Vec3) -> f,
            "The dot product: positive when the vectors point the same way, zero when they are at right angles, negative when they oppose.",
            "surface_normal:LINALG_Vec3 = linalg_vec3(0.0, 1.0, 0.0);\ntoward_light:LINALG_Vec3 = linalg_vec3(0.0, 1.0, 0.0);\nalignment:f = linalg_vec3_dot(surface_normal, toward_light);";
        "linalg_vec3_cross" => "std_lib::linalg::vec3_cross", (first: Vec3, second: Vec3) -> Vec3,
            "The vector at right angles to both - the direction a surface faces, given two directions lying in it.",
            "first_edge:LINALG_Vec3 = linalg_vec3(1.0, 0.0, 0.0);\nsecond_edge:LINALG_Vec3 = linalg_vec3(0.0, 1.0, 0.0);\nnormal:LINALG_Vec3 = linalg_vec3_cross(first_edge, second_edge);";
        "linalg_vec3_length" => "std_lib::linalg::vec3_length", (vector: Vec3) -> f,
            "How long the vector is.",
            "velocity:LINALG_Vec3 = linalg_vec3(0.0, 3.0, 4.0);\nspeed:f = linalg_vec3_length(velocity);";
        "linalg_vec3_length_squared" => "std_lib::linalg::vec3_length_squared", (vector: Vec3) -> f,
            "The length multiplied by itself, without the square root. Comparing two of these answers which vector is longer for less work.",
            "velocity:LINALG_Vec3 = linalg_vec3(0.0, 3.0, 4.0);\nrough:f = linalg_vec3_length_squared(velocity);";
        "linalg_vec3_distance" => "std_lib::linalg::vec3_distance", (first: Vec3, second: Vec3) -> f,
            "How far apart two points are.",
            "here:LINALG_Vec3 = linalg_vec3(0.0, 0.0, 0.0);\nthere:LINALG_Vec3 = linalg_vec3(1.0, 2.0, 2.0);\ngap:f = linalg_vec3_distance(here, there);";
        "linalg_vec3_normalize" => "std_lib::linalg::vec3_normalize", (vector: Vec3) -> (Vec3!e),
            "The vector of length one pointing the same way. A vector of zero length points in no direction, so that is an error.",
            "velocity:LINALG_Vec3 = linalg_vec3(0.0, 3.0, 4.0);\nheading:LINALG_Vec3 = danger(linalg_vec3_normalize(velocity));";
        "linalg_vec3_lerp" => "std_lib::linalg::vec3_lerp", (start: Vec3, end: Vec3, t: f) -> Vec3,
            "The point part of the way from start to end, with t clamped to 0.0..1.0.",
            "start:LINALG_Vec3 = linalg_vec3(0.0, 0.0, 0.0);\nfinish:LINALG_Vec3 = linalg_vec3(10.0, 4.0, 6.0);\nhalfway:LINALG_Vec3 = linalg_vec3_lerp(start, finish, 0.5);";
        "linalg_vec3_min" => "std_lib::linalg::vec3_min", (first: Vec3, second: Vec3) -> Vec3,
            "The smaller of each component - one corner of the box holding both points.",
            "first_corner:LINALG_Vec3 = linalg_vec3(4.0, 9.0, 2.0);\nsecond_corner:LINALG_Vec3 = linalg_vec3(1.0, 12.0, 5.0);\nlow_corner:LINALG_Vec3 = linalg_vec3_min(first_corner, second_corner);";
        "linalg_vec3_max" => "std_lib::linalg::vec3_max", (first: Vec3, second: Vec3) -> Vec3,
            "The larger of each component - the opposite corner of the box holding both points.",
            "first_corner:LINALG_Vec3 = linalg_vec3(4.0, 9.0, 2.0);\nsecond_corner:LINALG_Vec3 = linalg_vec3(1.0, 12.0, 5.0);\nhigh_corner:LINALG_Vec3 = linalg_vec3_max(first_corner, second_corner);";
        "linalg_vec3_clamp" => "std_lib::linalg::vec3_clamp", (vector: Vec3, low: Vec3, high: Vec3) -> Vec3,
            "Keeps a point inside a box, one component at a time.",
            "position:LINALG_Vec3 = linalg_vec3(1.0, 2.0, 3.0);\nlow_corner:LINALG_Vec3 = linalg_vec3(0.0, 0.0, 0.0);\nhigh_corner:LINALG_Vec3 = linalg_vec3(10.0, 10.0, 10.0);\ninside:LINALG_Vec3 = linalg_vec3_clamp(position, low_corner, high_corner);";
        "linalg_vec3_reflect" => "std_lib::linalg::vec3_reflect", (vector: Vec3, normal: Vec3) -> Vec3,
            "Bounces a vector off a surface facing the given direction - where a ball goes when it hits a wall. The normal should have length one.",
            "velocity:LINALG_Vec3 = linalg_vec3(0.0, 3.0, 4.0);\nsurface_normal:LINALG_Vec3 = linalg_vec3(0.0, 1.0, 0.0);\nbounced:LINALG_Vec3 = linalg_vec3_reflect(velocity, surface_normal);";
        "linalg_vec3_angle_between" => "std_lib::linalg::vec3_angle_between", (first: Vec3, second: Vec3) -> (f!e),
            "The angle between two vectors in radians, from 0.0 to pi. A vector of zero length has no angle, so that is an error.",
            "surface_normal:LINALG_Vec3 = linalg_vec3(0.0, 1.0, 0.0);\ntoward_light:LINALG_Vec3 = linalg_vec3(0.0, 1.0, 0.0);\nangle:f = danger(linalg_vec3_angle_between(surface_normal, toward_light));";
        "linalg_vec3_equals" => "std_lib::linalg::vec3_equals", (first: Vec3, second: Vec3, tolerance: f) -> b,
            "True when two vectors match to within the given tolerance, which is how floats have to be compared once any arithmetic has happened to them.",
            "computed:LINALG_Vec3 = linalg_vec3(1.0, 2.0, 3.0);\nexpected:LINALG_Vec3 = linalg_vec3(1.0, 2.0, 3.0);\nsame:b = linalg_vec3_equals(computed, expected, 0.0001);";
        "linalg_vec3_to_array" => "std_lib::linalg::vec3_to_array", (vector: Vec3) -> [f],
            "The three components as an array, in x, y, z order.",
            "position:LINALG_Vec3 = linalg_vec3(1.0, 2.0, 3.0);\ntriple:a:f = linalg_vec3_to_array(position);";
        "linalg_vec3_from_array" => "std_lib::linalg::vec3_from_array", (values: [f]) -> (Vec3!e),
            "A vector from an array of exactly three numbers.",
            "triple:a:f = [1.0, 2.0, 3.0];\nposition:LINALG_Vec3 = danger(linalg_vec3_from_array(triple));";

        // Mat3
        "linalg_mat3" => "std_lib::linalg::mat3", (values: [f]) -> (Mat3!e),
            "A 3x3 matrix from exactly nine numbers, read left to right and top to bottom.",
            "nine_numbers:a:f = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];\nmatrix:LINALG_Mat3 = danger(linalg_mat3(nine_numbers));";
        "linalg_mat3_identity" => "std_lib::linalg::mat3_identity", () -> Mat3,
            "The transform that changes nothing - the one to start building from.",
            "transform:LINALG_Mat3 = linalg_mat3_identity();";
        "linalg_mat3_translation" => "std_lib::linalg::mat3_translation", (x: f, y: f) -> Mat3,
            "The transform that moves everything by the given amounts.",
            "shift:LINALG_Mat3 = linalg_mat3_translation(40.0, 20.0);";
        "linalg_mat3_rotation" => "std_lib::linalg::mat3_rotation", (radians: f) -> Mat3,
            "The transform that turns everything about the origin. To turn about another point, translate it to the origin first and back afterwards.",
            "turn:LINALG_Mat3 = linalg_mat3_rotation(math_to_radians(45.0));";
        "linalg_mat3_scaling" => "std_lib::linalg::mat3_scaling", (x: f, y: f) -> Mat3,
            "The transform that stretches everything about the origin, with a separate factor per axis.",
            "grow:LINALG_Mat3 = linalg_mat3_scaling(2.0, 2.0);";
        "linalg_mat3_multiply" => "std_lib::linalg::mat3_multiply", (first: Mat3, second: Mat3) -> Mat3,
            "Combines two transforms into one. The second happens first, which is the order the notation has meant since before computers.",
            "shift:LINALG_Mat3 = linalg_mat3_translation(40.0, 20.0);\nturn:LINALG_Mat3 = linalg_mat3_rotation(math_to_radians(45.0));\ncombined:LINALG_Mat3 = linalg_mat3_multiply(shift, turn);";
        "linalg_mat3_transform_point" => "std_lib::linalg::mat3_transform_point", (matrix: Mat3, point: Vec2) -> Vec2,
            "Moves a point through the transform, translation included.",
            "transform:LINALG_Mat3 = linalg_mat3_translation(40.0, 20.0);\ncorner:LINALG_Vec2 = linalg_vec2(10.0, 10.0);\nmoved:LINALG_Vec2 = linalg_mat3_transform_point(transform, corner);";
        "linalg_mat3_transform_vector" => "std_lib::linalg::mat3_transform_vector", (matrix: Mat3, vector: Vec2) -> Vec2,
            "Moves a direction through the transform, ignoring translation - a direction has no position, so shifting one is always a mistake.",
            "transform:LINALG_Mat3 = linalg_mat3_translation(40.0, 20.0);\nheading:LINALG_Vec2 = linalg_vec2(1.0, 0.0);\nturned:LINALG_Vec2 = linalg_mat3_transform_vector(transform, heading);";
        "linalg_mat3_transpose" => "std_lib::linalg::mat3_transpose", (matrix: Mat3) -> Mat3,
            "The matrix with its rows and columns swapped.",
            "matrix:LINALG_Mat3 = linalg_mat3_identity();\nflipped:LINALG_Mat3 = linalg_mat3_transpose(matrix);";
        "linalg_mat3_determinant" => "std_lib::linalg::mat3_determinant", (matrix: Mat3) -> f,
            "How much the transform multiplies area by. Zero means it flattens the plane onto a line, which is exactly when it cannot be undone.",
            "transform:LINALG_Mat3 = linalg_mat3_translation(40.0, 20.0);\narea_factor:f = linalg_mat3_determinant(transform);";
        "linalg_mat3_inverse" => "std_lib::linalg::mat3_inverse", (matrix: Mat3) -> (Mat3!e),
            "The transform that undoes this one. A transform that flattens the plane has no inverse, so that is an error.",
            "transform:LINALG_Mat3 = linalg_mat3_translation(40.0, 20.0);\nundo:LINALG_Mat3 = danger(linalg_mat3_inverse(transform));";
        "linalg_mat3_get" => "std_lib::linalg::mat3_get", (matrix: Mat3, row: i, column: i) -> (f!e),
            "One value out of the matrix, by row and column, both counted from 0.",
            "matrix:LINALG_Mat3 = linalg_mat3_identity();\nvalue:f = danger(linalg_mat3_get(matrix, 1, 2));";
        "linalg_mat3_to_array" => "std_lib::linalg::mat3_to_array", (matrix: Mat3) -> [f],
            "All nine values as an array, read left to right and top to bottom.",
            "transform:LINALG_Mat3 = linalg_mat3_translation(40.0, 20.0);\nvalues:a:f = linalg_mat3_to_array(transform);";
        "linalg_mat3_equals" => "std_lib::linalg::mat3_equals", (first: Mat3, second: Mat3, tolerance: f) -> b,
            "True when two matrices match to within the given tolerance, which is how floats have to be compared once any arithmetic has happened to them.",
            "computed:LINALG_Mat3 = linalg_mat3_identity();\nexpected:LINALG_Mat3 = linalg_mat3_identity();\nsame:b = linalg_mat3_equals(computed, expected, 0.0001);";
    }
}
