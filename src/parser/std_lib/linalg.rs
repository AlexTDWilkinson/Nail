//! Vectors and matrices - the arithmetic of position, direction and shape.
//!
//! A point on a chart, the direction one place lies from another, the corner
//! of a rectangle, the transform that moves a whole drawing at once: all of it
//! is two or three numbers travelling together, and all of it is done wrong at
//! least once when those numbers travel separately. `LINALG_Vec2` and
//! `LINALG_Vec3` keep them together, so a function that wants a position takes
//! one argument instead of two and cannot be handed them backwards.
//!
//! Every operation is a function from values to values, like the rest of Nail.
//! Nothing here mutates a vector in place; `linalg_vec2_add` returns a new
//! vector and leaves both arguments alone.
//!
//! Only the operations that can genuinely fail return a result: dividing by a
//! zero component, normalizing a vector with no length, asking the angle of a
//! vector that points nowhere, inverting a matrix that flattens the plane.
//! Adding, scaling and multiplying cannot fail, so they do not make a program
//! unwrap anything.
//!
//! `LINALG_Mat3` is a 3x3 matrix stored row-major in nine floats, which is the
//! shape every 2D transform takes: rotation, scaling and translation together
//! in one value that composes with `linalg_mat3_multiply`. It pairs with
//! `draw_*`, whose coordinates are the same plane.

use serde::{Deserialize, Serialize};

/// A point or direction in the plane.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LINALG_Vec2 {
    pub x_coordinate: f64,
    pub y_coordinate: f64,
}

/// A point or direction in space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LINALG_Vec3 {
    pub x_coordinate: f64,
    pub y_coordinate: f64,
    pub z_coordinate: f64,
}

/// A 3x3 matrix, stored row-major: the first three values are the top row.
///
/// The bottom row of a 2D transform is always `0 0 1`, but it is stored anyway
/// so that multiplying two matrices is the plain textbook loop rather than a
/// special case, and so a matrix built by hand from nine numbers behaves the
/// same as one built by `linalg_mat3_rotation`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LINALG_Mat3 {
    pub values: Vec<f64>,
}

// ---------------------------------------------------------------------------
// Vec2
// ---------------------------------------------------------------------------

pub fn vec2(x: f64, y: f64) -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: x, y_coordinate: y };
}

pub fn vec2_zero() -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: 0.0, y_coordinate: 0.0 };
}

pub fn vec2_add(first: LINALG_Vec2, second: LINALG_Vec2) -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: first.x_coordinate + second.x_coordinate, y_coordinate: first.y_coordinate + second.y_coordinate };
}

pub fn vec2_subtract(first: LINALG_Vec2, second: LINALG_Vec2) -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: first.x_coordinate - second.x_coordinate, y_coordinate: first.y_coordinate - second.y_coordinate };
}

pub fn vec2_scale(vector: LINALG_Vec2, factor: f64) -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: vector.x_coordinate * factor, y_coordinate: vector.y_coordinate * factor };
}

pub fn vec2_multiply(first: LINALG_Vec2, second: LINALG_Vec2) -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: first.x_coordinate * second.x_coordinate, y_coordinate: first.y_coordinate * second.y_coordinate };
}

pub fn vec2_divide(first: LINALG_Vec2, second: LINALG_Vec2) -> Result<LINALG_Vec2, String> {
    if second.x_coordinate == 0.0 || second.y_coordinate == 0.0 {
        return Err(format!("linalg_vec2_divide: cannot divide by a vector with a zero component ({}, {})", second.x_coordinate, second.y_coordinate));
    }
    return Ok(LINALG_Vec2 { x_coordinate: first.x_coordinate / second.x_coordinate, y_coordinate: first.y_coordinate / second.y_coordinate });
}

pub fn vec2_negate(vector: LINALG_Vec2) -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: -vector.x_coordinate, y_coordinate: -vector.y_coordinate };
}

pub fn vec2_dot(first: LINALG_Vec2, second: LINALG_Vec2) -> f64 {
    return first.x_coordinate * second.x_coordinate + first.y_coordinate * second.y_coordinate;
}

pub fn vec2_length(vector: LINALG_Vec2) -> f64 {
    return vector.x_coordinate.hypot(vector.y_coordinate);
}

pub fn vec2_length_squared(vector: LINALG_Vec2) -> f64 {
    return vector.x_coordinate * vector.x_coordinate + vector.y_coordinate * vector.y_coordinate;
}

pub fn vec2_distance(first: LINALG_Vec2, second: LINALG_Vec2) -> f64 {
    return (first.x_coordinate - second.x_coordinate).hypot(first.y_coordinate - second.y_coordinate);
}

pub fn vec2_normalize(vector: LINALG_Vec2) -> Result<LINALG_Vec2, String> {
    let length = vec2_length(vector);
    if length == 0.0 || !length.is_finite() {
        return Err("linalg_vec2_normalize: a vector of zero length points in no direction".to_string());
    }
    return Ok(LINALG_Vec2 { x_coordinate: vector.x_coordinate / length, y_coordinate: vector.y_coordinate / length });
}

/// The vector at a right angle to this one, turned a quarter turn the way the
/// coordinates grow.
pub fn vec2_perpendicular(vector: LINALG_Vec2) -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: -vector.y_coordinate, y_coordinate: vector.x_coordinate };
}

/// Turns a vector about the origin, in radians.
pub fn vec2_rotate(vector: LINALG_Vec2, radians: f64) -> LINALG_Vec2 {
    let (sine, cosine) = radians.sin_cos();
    return LINALG_Vec2 { x_coordinate: vector.x_coordinate * cosine - vector.y_coordinate * sine, y_coordinate: vector.x_coordinate * sine + vector.y_coordinate * cosine };
}

pub fn vec2_lerp(start: LINALG_Vec2, end: LINALG_Vec2, t: f64) -> LINALG_Vec2 {
    let amount = t.clamp(0.0, 1.0);
    return LINALG_Vec2 { x_coordinate: start.x_coordinate + (end.x_coordinate - start.x_coordinate) * amount, y_coordinate: start.y_coordinate + (end.y_coordinate - start.y_coordinate) * amount };
}

pub fn vec2_min(first: LINALG_Vec2, second: LINALG_Vec2) -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: first.x_coordinate.min(second.x_coordinate), y_coordinate: first.y_coordinate.min(second.y_coordinate) };
}

pub fn vec2_max(first: LINALG_Vec2, second: LINALG_Vec2) -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: first.x_coordinate.max(second.x_coordinate), y_coordinate: first.y_coordinate.max(second.y_coordinate) };
}

pub fn vec2_clamp(vector: LINALG_Vec2, low: LINALG_Vec2, high: LINALG_Vec2) -> LINALG_Vec2 {
    return LINALG_Vec2 { x_coordinate: vector.x_coordinate.clamp(low.x_coordinate.min(high.x_coordinate), high.x_coordinate.max(low.x_coordinate)), y_coordinate: vector.y_coordinate.clamp(low.y_coordinate.min(high.y_coordinate), high.y_coordinate.max(low.y_coordinate)) };
}

/// Bounces a vector off a surface with the given normal - the direction a ball
/// leaves a wall it arrived at.
pub fn vec2_reflect(vector: LINALG_Vec2, normal: LINALG_Vec2) -> LINALG_Vec2 {
    let twice_projection = 2.0 * vec2_dot(vector, normal);
    return LINALG_Vec2 { x_coordinate: vector.x_coordinate - twice_projection * normal.x_coordinate, y_coordinate: vector.y_coordinate - twice_projection * normal.y_coordinate };
}

pub fn vec2_angle_between(first: LINALG_Vec2, second: LINALG_Vec2) -> Result<f64, String> {
    let lengths = vec2_length(first) * vec2_length(second);
    if lengths == 0.0 || !lengths.is_finite() {
        return Err("linalg_vec2_angle_between: a vector of zero length has no angle".to_string());
    }
    return Ok((vec2_dot(first, second) / lengths).clamp(-1.0, 1.0).acos());
}

/// True when two vectors are the same to within the given tolerance, which is
/// how floats have to be compared once any arithmetic has happened to them.
pub fn vec2_equals(first: LINALG_Vec2, second: LINALG_Vec2, tolerance: f64) -> bool {
    let allowed = tolerance.abs();
    return (first.x_coordinate - second.x_coordinate).abs() <= allowed && (first.y_coordinate - second.y_coordinate).abs() <= allowed;
}

pub fn vec2_to_array(vector: LINALG_Vec2) -> Vec<f64> {
    return vec![vector.x_coordinate, vector.y_coordinate];
}

pub fn vec2_from_array(values: Vec<f64>) -> Result<LINALG_Vec2, String> {
    if values.len() != 2 {
        return Err(format!("linalg_vec2_from_array: a 2D vector needs exactly 2 numbers, got {}", values.len()));
    }
    return Ok(LINALG_Vec2 { x_coordinate: values[0], y_coordinate: values[1] });
}

// ---------------------------------------------------------------------------
// Vec3
// ---------------------------------------------------------------------------

pub fn vec3(x: f64, y: f64, z: f64) -> LINALG_Vec3 {
    return LINALG_Vec3 { x_coordinate: x, y_coordinate: y, z_coordinate: z };
}

pub fn vec3_zero() -> LINALG_Vec3 {
    return LINALG_Vec3 { x_coordinate: 0.0, y_coordinate: 0.0, z_coordinate: 0.0 };
}

pub fn vec3_add(first: LINALG_Vec3, second: LINALG_Vec3) -> LINALG_Vec3 {
    return LINALG_Vec3 { x_coordinate: first.x_coordinate + second.x_coordinate, y_coordinate: first.y_coordinate + second.y_coordinate, z_coordinate: first.z_coordinate + second.z_coordinate };
}

pub fn vec3_subtract(first: LINALG_Vec3, second: LINALG_Vec3) -> LINALG_Vec3 {
    return LINALG_Vec3 { x_coordinate: first.x_coordinate - second.x_coordinate, y_coordinate: first.y_coordinate - second.y_coordinate, z_coordinate: first.z_coordinate - second.z_coordinate };
}

pub fn vec3_scale(vector: LINALG_Vec3, factor: f64) -> LINALG_Vec3 {
    return LINALG_Vec3 { x_coordinate: vector.x_coordinate * factor, y_coordinate: vector.y_coordinate * factor, z_coordinate: vector.z_coordinate * factor };
}

pub fn vec3_multiply(first: LINALG_Vec3, second: LINALG_Vec3) -> LINALG_Vec3 {
    return LINALG_Vec3 { x_coordinate: first.x_coordinate * second.x_coordinate, y_coordinate: first.y_coordinate * second.y_coordinate, z_coordinate: first.z_coordinate * second.z_coordinate };
}

pub fn vec3_divide(first: LINALG_Vec3, second: LINALG_Vec3) -> Result<LINALG_Vec3, String> {
    if second.x_coordinate == 0.0 || second.y_coordinate == 0.0 || second.z_coordinate == 0.0 {
        return Err(format!("linalg_vec3_divide: cannot divide by a vector with a zero component ({}, {}, {})", second.x_coordinate, second.y_coordinate, second.z_coordinate));
    }
    return Ok(LINALG_Vec3 { x_coordinate: first.x_coordinate / second.x_coordinate, y_coordinate: first.y_coordinate / second.y_coordinate, z_coordinate: first.z_coordinate / second.z_coordinate });
}

pub fn vec3_negate(vector: LINALG_Vec3) -> LINALG_Vec3 {
    return LINALG_Vec3 { x_coordinate: -vector.x_coordinate, y_coordinate: -vector.y_coordinate, z_coordinate: -vector.z_coordinate };
}

pub fn vec3_dot(first: LINALG_Vec3, second: LINALG_Vec3) -> f64 {
    return first.x_coordinate * second.x_coordinate + first.y_coordinate * second.y_coordinate + first.z_coordinate * second.z_coordinate;
}

/// The vector at right angles to both - the direction a surface faces, given
/// two directions lying in it.
pub fn vec3_cross(first: LINALG_Vec3, second: LINALG_Vec3) -> LINALG_Vec3 {
    return LINALG_Vec3 {
        x_coordinate: first.y_coordinate * second.z_coordinate - first.z_coordinate * second.y_coordinate,
        y_coordinate: first.z_coordinate * second.x_coordinate - first.x_coordinate * second.z_coordinate,
        z_coordinate: first.x_coordinate * second.y_coordinate - first.y_coordinate * second.x_coordinate,
    };
}

pub fn vec3_length(vector: LINALG_Vec3) -> f64 {
    return vec3_length_squared(vector).sqrt();
}

pub fn vec3_length_squared(vector: LINALG_Vec3) -> f64 {
    return vector.x_coordinate * vector.x_coordinate + vector.y_coordinate * vector.y_coordinate + vector.z_coordinate * vector.z_coordinate;
}

pub fn vec3_distance(first: LINALG_Vec3, second: LINALG_Vec3) -> f64 {
    return vec3_length(vec3_subtract(first, second));
}

pub fn vec3_normalize(vector: LINALG_Vec3) -> Result<LINALG_Vec3, String> {
    let length = vec3_length(vector);
    if length == 0.0 || !length.is_finite() {
        return Err("linalg_vec3_normalize: a vector of zero length points in no direction".to_string());
    }
    return Ok(LINALG_Vec3 { x_coordinate: vector.x_coordinate / length, y_coordinate: vector.y_coordinate / length, z_coordinate: vector.z_coordinate / length });
}

pub fn vec3_lerp(start: LINALG_Vec3, end: LINALG_Vec3, t: f64) -> LINALG_Vec3 {
    let amount = t.clamp(0.0, 1.0);
    return LINALG_Vec3 {
        x_coordinate: start.x_coordinate + (end.x_coordinate - start.x_coordinate) * amount,
        y_coordinate: start.y_coordinate + (end.y_coordinate - start.y_coordinate) * amount,
        z_coordinate: start.z_coordinate + (end.z_coordinate - start.z_coordinate) * amount,
    };
}

pub fn vec3_min(first: LINALG_Vec3, second: LINALG_Vec3) -> LINALG_Vec3 {
    return LINALG_Vec3 { x_coordinate: first.x_coordinate.min(second.x_coordinate), y_coordinate: first.y_coordinate.min(second.y_coordinate), z_coordinate: first.z_coordinate.min(second.z_coordinate) };
}

pub fn vec3_max(first: LINALG_Vec3, second: LINALG_Vec3) -> LINALG_Vec3 {
    return LINALG_Vec3 { x_coordinate: first.x_coordinate.max(second.x_coordinate), y_coordinate: first.y_coordinate.max(second.y_coordinate), z_coordinate: first.z_coordinate.max(second.z_coordinate) };
}

pub fn vec3_clamp(vector: LINALG_Vec3, low: LINALG_Vec3, high: LINALG_Vec3) -> LINALG_Vec3 {
    return LINALG_Vec3 {
        x_coordinate: vector.x_coordinate.clamp(low.x_coordinate.min(high.x_coordinate), high.x_coordinate.max(low.x_coordinate)),
        y_coordinate: vector.y_coordinate.clamp(low.y_coordinate.min(high.y_coordinate), high.y_coordinate.max(low.y_coordinate)),
        z_coordinate: vector.z_coordinate.clamp(low.z_coordinate.min(high.z_coordinate), high.z_coordinate.max(low.z_coordinate)),
    };
}

pub fn vec3_reflect(vector: LINALG_Vec3, normal: LINALG_Vec3) -> LINALG_Vec3 {
    let twice_projection = 2.0 * vec3_dot(vector, normal);
    return vec3_subtract(vector, vec3_scale(normal, twice_projection));
}

pub fn vec3_angle_between(first: LINALG_Vec3, second: LINALG_Vec3) -> Result<f64, String> {
    let lengths = vec3_length(first) * vec3_length(second);
    if lengths == 0.0 || !lengths.is_finite() {
        return Err("linalg_vec3_angle_between: a vector of zero length has no angle".to_string());
    }
    return Ok((vec3_dot(first, second) / lengths).clamp(-1.0, 1.0).acos());
}

pub fn vec3_equals(first: LINALG_Vec3, second: LINALG_Vec3, tolerance: f64) -> bool {
    let allowed = tolerance.abs();
    return (first.x_coordinate - second.x_coordinate).abs() <= allowed && (first.y_coordinate - second.y_coordinate).abs() <= allowed && (first.z_coordinate - second.z_coordinate).abs() <= allowed;
}

pub fn vec3_to_array(vector: LINALG_Vec3) -> Vec<f64> {
    return vec![vector.x_coordinate, vector.y_coordinate, vector.z_coordinate];
}

pub fn vec3_from_array(values: Vec<f64>) -> Result<LINALG_Vec3, String> {
    if values.len() != 3 {
        return Err(format!("linalg_vec3_from_array: a 3D vector needs exactly 3 numbers, got {}", values.len()));
    }
    return Ok(LINALG_Vec3 { x_coordinate: values[0], y_coordinate: values[1], z_coordinate: values[2] });
}

// ---------------------------------------------------------------------------
// Mat3
// ---------------------------------------------------------------------------

/// Builds a matrix from nine numbers, read left to right and top to bottom.
pub fn mat3(values: Vec<f64>) -> Result<LINALG_Mat3, String> {
    if values.len() != 9 {
        return Err(format!("linalg_mat3: a 3x3 matrix needs exactly 9 numbers, got {}", values.len()));
    }
    return Ok(LINALG_Mat3 { values });
}

/// The transform that changes nothing.
pub fn mat3_identity() -> LINALG_Mat3 {
    return LINALG_Mat3 { values: vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] };
}

pub fn mat3_translation(x: f64, y: f64) -> LINALG_Mat3 {
    return LINALG_Mat3 { values: vec![1.0, 0.0, x, 0.0, 1.0, y, 0.0, 0.0, 1.0] };
}

/// A turn about the origin, in radians. To turn about another point, translate
/// it to the origin first and back afterwards.
pub fn mat3_rotation(radians: f64) -> LINALG_Mat3 {
    let (sine, cosine) = radians.sin_cos();
    return LINALG_Mat3 { values: vec![cosine, -sine, 0.0, sine, cosine, 0.0, 0.0, 0.0, 1.0] };
}

pub fn mat3_scaling(x: f64, y: f64) -> LINALG_Mat3 {
    return LINALG_Mat3 { values: vec![x, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 1.0] };
}

/// Combines two transforms into one. The right-hand transform happens first,
/// which is the order the notation has meant since before computers.
pub fn mat3_multiply(first: LINALG_Mat3, second: LINALG_Mat3) -> LINALG_Mat3 {
    let mut values = vec![0.0; 9];
    for row in 0..3 {
        for column in 0..3 {
            let mut total = 0.0;
            for index in 0..3 {
                total += at(&first, row, index) * at(&second, index, column);
            }
            values[row * 3 + column] = total;
        }
    }
    return LINALG_Mat3 { values };
}

/// Moves a point through the transform, translation included.
pub fn mat3_transform_point(matrix: LINALG_Mat3, point: LINALG_Vec2) -> LINALG_Vec2 {
    return LINALG_Vec2 {
        x_coordinate: at(&matrix, 0, 0) * point.x_coordinate + at(&matrix, 0, 1) * point.y_coordinate + at(&matrix, 0, 2),
        y_coordinate: at(&matrix, 1, 0) * point.x_coordinate + at(&matrix, 1, 1) * point.y_coordinate + at(&matrix, 1, 2),
    };
}

/// Moves a direction through the transform, ignoring translation - a direction
/// has no position to move, so shifting one is always a bug.
pub fn mat3_transform_vector(matrix: LINALG_Mat3, vector: LINALG_Vec2) -> LINALG_Vec2 {
    return LINALG_Vec2 {
        x_coordinate: at(&matrix, 0, 0) * vector.x_coordinate + at(&matrix, 0, 1) * vector.y_coordinate,
        y_coordinate: at(&matrix, 1, 0) * vector.x_coordinate + at(&matrix, 1, 1) * vector.y_coordinate,
    };
}

pub fn mat3_transpose(matrix: LINALG_Mat3) -> LINALG_Mat3 {
    let mut values = vec![0.0; 9];
    for row in 0..3 {
        for column in 0..3 {
            values[column * 3 + row] = at(&matrix, row, column);
        }
    }
    return LINALG_Mat3 { values };
}

/// How much the transform multiplies area by. Zero means it flattens the plane
/// onto a line, which is exactly when it cannot be undone.
pub fn mat3_determinant(matrix: LINALG_Mat3) -> f64 {
    return at(&matrix, 0, 0) * (at(&matrix, 1, 1) * at(&matrix, 2, 2) - at(&matrix, 1, 2) * at(&matrix, 2, 1))
        - at(&matrix, 0, 1) * (at(&matrix, 1, 0) * at(&matrix, 2, 2) - at(&matrix, 1, 2) * at(&matrix, 2, 0))
        + at(&matrix, 0, 2) * (at(&matrix, 1, 0) * at(&matrix, 2, 1) - at(&matrix, 1, 1) * at(&matrix, 2, 0));
}

/// The transform that undoes this one.
pub fn mat3_inverse(matrix: LINALG_Mat3) -> Result<LINALG_Mat3, String> {
    let determinant = mat3_determinant(matrix.clone());
    if determinant == 0.0 || !determinant.is_finite() {
        return Err("linalg_mat3_inverse: this transform flattens the plane, so nothing can undo it".to_string());
    }

    let mut values = vec![0.0; 9];
    for row in 0..3 {
        for column in 0..3 {
            // The cofactor of (column, row) rather than (row, column): the
            // inverse is the transposed cofactor matrix over the determinant.
            let (first_row, second_row) = other_two(column);
            let (first_column, second_column) = other_two(row);
            let minor = at(&matrix, first_row, first_column) * at(&matrix, second_row, second_column)
                - at(&matrix, first_row, second_column) * at(&matrix, second_row, first_column);
            let sign = if (row + column) % 2 == 0 { 1.0 } else { -1.0 };
            values[row * 3 + column] = sign * minor / determinant;
        }
    }
    return Ok(LINALG_Mat3 { values });
}

/// One value out of the matrix, by row and column, both counted from 0.
pub fn mat3_get(matrix: LINALG_Mat3, row: i64, column: i64) -> Result<f64, String> {
    if !(0..3).contains(&row) || !(0..3).contains(&column) {
        return Err(format!("linalg_mat3_get: a 3x3 matrix has rows and columns 0 to 2, not row {} column {}", row, column));
    }
    return Ok(matrix.values[(row * 3 + column) as usize]);
}

pub fn mat3_to_array(matrix: LINALG_Mat3) -> Vec<f64> {
    return matrix.values;
}

pub fn mat3_equals(first: LINALG_Mat3, second: LINALG_Mat3, tolerance: f64) -> bool {
    let allowed = tolerance.abs();
    return first.values.iter().zip(second.values.iter()).all(|(left, right)| (left - right).abs() <= allowed);
}

/// Row-major lookup. Every matrix reaching here came from `mat3` or from a
/// builder above, so the length is nine and the index is in range.
fn at(matrix: &LINALG_Mat3, row: usize, column: usize) -> f64 {
    return matrix.values[row * 3 + column];
}

/// The two indices that are not this one, in order - the rows or columns a
/// minor is taken from.
fn other_two(index: usize) -> (usize, usize) {
    return match index {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 1e-9;
    }

    #[test]
    fn vec2_arithmetic_is_componentwise() {
        let sum = vec2_add(vec2(1.0, 2.0), vec2(3.0, 4.0));
        assert_eq!(sum, vec2(4.0, 6.0));
        assert_eq!(vec2_subtract(sum, vec2(3.0, 4.0)), vec2(1.0, 2.0));
        assert_eq!(vec2_scale(vec2(1.0, 2.0), 3.0), vec2(3.0, 6.0));
        assert_eq!(vec2_multiply(vec2(2.0, 3.0), vec2(4.0, 5.0)), vec2(8.0, 15.0));
        assert_eq!(vec2_negate(vec2(1.0, -2.0)), vec2(-1.0, 2.0));
    }

    #[test]
    fn vec2_divide_rejects_a_zero_component() {
        assert!(vec2_divide(vec2(1.0, 1.0), vec2(0.0, 1.0)).is_err());
        assert_eq!(vec2_divide(vec2(6.0, 8.0), vec2(2.0, 4.0)).unwrap(), vec2(3.0, 2.0));
    }

    #[test]
    fn vec2_length_and_normalize_agree() {
        assert!(close(vec2_length(vec2(3.0, 4.0)), 5.0));
        assert!(close(vec2_length_squared(vec2(3.0, 4.0)), 25.0));
        assert!(close(vec2_distance(vec2(0.0, 0.0), vec2(3.0, 4.0)), 5.0));
        let unit = vec2_normalize(vec2(3.0, 4.0)).unwrap();
        assert!(close(vec2_length(unit), 1.0));
        assert!(vec2_normalize(vec2_zero()).is_err());
    }

    #[test]
    fn vec2_dot_and_angle_describe_the_same_turn() {
        assert!(close(vec2_dot(vec2(1.0, 0.0), vec2(0.0, 1.0)), 0.0));
        let right_angle = vec2_angle_between(vec2(1.0, 0.0), vec2(0.0, 2.0)).unwrap();
        assert!(close(right_angle, std::f64::consts::FRAC_PI_2));
        assert!(vec2_angle_between(vec2_zero(), vec2(1.0, 0.0)).is_err());
    }

    #[test]
    fn vec2_rotate_and_perpendicular_turn_a_quarter_circle() {
        let turned = vec2_rotate(vec2(1.0, 0.0), std::f64::consts::FRAC_PI_2);
        assert!(vec2_equals(turned, vec2(0.0, 1.0), 1e-9));
        assert_eq!(vec2_perpendicular(vec2(1.0, 0.0)), vec2(0.0, 1.0));
    }

    #[test]
    fn vec2_reflect_bounces_off_a_wall() {
        let bounced = vec2_reflect(vec2(1.0, -1.0), vec2(0.0, 1.0));
        assert!(vec2_equals(bounced, vec2(1.0, 1.0), 1e-9));
    }

    #[test]
    fn vec2_lerp_clamps_its_amount() {
        assert_eq!(vec2_lerp(vec2(0.0, 0.0), vec2(10.0, 20.0), 0.5), vec2(5.0, 10.0));
        assert_eq!(vec2_lerp(vec2(0.0, 0.0), vec2(10.0, 20.0), 2.0), vec2(10.0, 20.0));
        assert_eq!(vec2_lerp(vec2(0.0, 0.0), vec2(10.0, 20.0), -1.0), vec2(0.0, 0.0));
    }

    #[test]
    fn vec2_bounds_are_componentwise() {
        assert_eq!(vec2_min(vec2(1.0, 5.0), vec2(3.0, 2.0)), vec2(1.0, 2.0));
        assert_eq!(vec2_max(vec2(1.0, 5.0), vec2(3.0, 2.0)), vec2(3.0, 5.0));
        assert_eq!(vec2_clamp(vec2(-1.0, 9.0), vec2(0.0, 0.0), vec2(5.0, 5.0)), vec2(0.0, 5.0));
    }

    #[test]
    fn vec2_arrays_round_trip() {
        assert_eq!(vec2_to_array(vec2(1.0, 2.0)), vec![1.0, 2.0]);
        assert_eq!(vec2_from_array(vec![1.0, 2.0]).unwrap(), vec2(1.0, 2.0));
        assert!(vec2_from_array(vec![1.0]).is_err());
        assert!(vec2_from_array(vec![1.0, 2.0, 3.0]).is_err());
    }

    #[test]
    fn vec3_cross_is_perpendicular_to_both() {
        let normal = vec3_cross(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0));
        assert_eq!(normal, vec3(0.0, 0.0, 1.0));
        assert!(close(vec3_dot(normal, vec3(1.0, 0.0, 0.0)), 0.0));
        assert!(close(vec3_dot(normal, vec3(0.0, 1.0, 0.0)), 0.0));
    }

    #[test]
    fn vec3_length_and_normalize_agree() {
        assert!(close(vec3_length(vec3(2.0, 3.0, 6.0)), 7.0));
        assert!(close(vec3_length_squared(vec3(2.0, 3.0, 6.0)), 49.0));
        assert!(close(vec3_distance(vec3(0.0, 0.0, 0.0), vec3(2.0, 3.0, 6.0)), 7.0));
        let unit = vec3_normalize(vec3(2.0, 3.0, 6.0)).unwrap();
        assert!(close(vec3_length(unit), 1.0));
        assert!(vec3_normalize(vec3_zero()).is_err());
    }

    #[test]
    fn vec3_arithmetic_is_componentwise() {
        assert_eq!(vec3_add(vec3(1.0, 2.0, 3.0), vec3(4.0, 5.0, 6.0)), vec3(5.0, 7.0, 9.0));
        assert_eq!(vec3_subtract(vec3(4.0, 5.0, 6.0), vec3(1.0, 2.0, 3.0)), vec3(3.0, 3.0, 3.0));
        assert_eq!(vec3_scale(vec3(1.0, 2.0, 3.0), 2.0), vec3(2.0, 4.0, 6.0));
        assert_eq!(vec3_multiply(vec3(1.0, 2.0, 3.0), vec3(4.0, 5.0, 6.0)), vec3(4.0, 10.0, 18.0));
        assert_eq!(vec3_negate(vec3(1.0, -2.0, 3.0)), vec3(-1.0, 2.0, -3.0));
        assert!(vec3_divide(vec3(1.0, 1.0, 1.0), vec3(1.0, 0.0, 1.0)).is_err());
        assert_eq!(vec3_divide(vec3(6.0, 8.0, 10.0), vec3(2.0, 4.0, 5.0)).unwrap(), vec3(3.0, 2.0, 2.0));
    }

    #[test]
    fn vec3_reflect_and_angle_behave() {
        let bounced = vec3_reflect(vec3(1.0, -1.0, 0.0), vec3(0.0, 1.0, 0.0));
        assert!(vec3_equals(bounced, vec3(1.0, 1.0, 0.0), 1e-9));
        let right_angle = vec3_angle_between(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)).unwrap();
        assert!(close(right_angle, std::f64::consts::FRAC_PI_2));
        assert!(vec3_angle_between(vec3_zero(), vec3(1.0, 0.0, 0.0)).is_err());
    }

    #[test]
    fn vec3_bounds_and_lerp_are_componentwise() {
        assert_eq!(vec3_min(vec3(1.0, 5.0, 3.0), vec3(3.0, 2.0, 4.0)), vec3(1.0, 2.0, 3.0));
        assert_eq!(vec3_max(vec3(1.0, 5.0, 3.0), vec3(3.0, 2.0, 4.0)), vec3(3.0, 5.0, 4.0));
        assert_eq!(vec3_clamp(vec3(-1.0, 9.0, 2.0), vec3_zero(), vec3(5.0, 5.0, 5.0)), vec3(0.0, 5.0, 2.0));
        assert_eq!(vec3_lerp(vec3_zero(), vec3(10.0, 20.0, 30.0), 0.5), vec3(5.0, 10.0, 15.0));
    }

    #[test]
    fn vec3_arrays_round_trip() {
        assert_eq!(vec3_to_array(vec3(1.0, 2.0, 3.0)), vec![1.0, 2.0, 3.0]);
        assert_eq!(vec3_from_array(vec![1.0, 2.0, 3.0]).unwrap(), vec3(1.0, 2.0, 3.0));
        assert!(vec3_from_array(vec![1.0, 2.0]).is_err());
    }

    #[test]
    fn mat3_rejects_the_wrong_number_of_values() {
        assert!(mat3(vec![1.0; 8]).is_err());
        assert!(mat3(vec![1.0; 10]).is_err());
        assert!(mat3(vec![1.0; 9]).is_ok());
    }

    #[test]
    fn mat3_identity_leaves_a_point_alone() {
        let point = vec2(3.0, 4.0);
        assert_eq!(mat3_transform_point(mat3_identity(), point), point);
    }

    #[test]
    fn mat3_translation_moves_points_but_not_directions() {
        let moved = mat3_translation(10.0, 20.0);
        assert_eq!(mat3_transform_point(moved.clone(), vec2(1.0, 2.0)), vec2(11.0, 22.0));
        assert_eq!(mat3_transform_vector(moved, vec2(1.0, 2.0)), vec2(1.0, 2.0));
    }

    #[test]
    fn mat3_multiply_applies_the_right_hand_transform_first() {
        // Scale then translate: the translation must not be scaled.
        let combined = mat3_multiply(mat3_translation(10.0, 0.0), mat3_scaling(2.0, 2.0));
        assert_eq!(mat3_transform_point(combined, vec2(1.0, 0.0)), vec2(12.0, 0.0));

        // The other order scales the translation with everything else.
        let other = mat3_multiply(mat3_scaling(2.0, 2.0), mat3_translation(10.0, 0.0));
        assert_eq!(mat3_transform_point(other, vec2(1.0, 0.0)), vec2(22.0, 0.0));
    }

    #[test]
    fn mat3_rotation_turns_a_quarter_circle() {
        let turned = mat3_transform_point(mat3_rotation(std::f64::consts::FRAC_PI_2), vec2(1.0, 0.0));
        assert!(vec2_equals(turned, vec2(0.0, 1.0), 1e-9));
    }

    #[test]
    fn mat3_inverse_undoes_the_transform() {
        let transform = mat3_multiply(mat3_translation(5.0, -3.0), mat3_multiply(mat3_rotation(0.7), mat3_scaling(2.0, 4.0)));
        let undo = mat3_inverse(transform.clone()).unwrap();
        let point = vec2(3.0, 7.0);
        let there_and_back = mat3_transform_point(undo.clone(), mat3_transform_point(transform.clone(), point));
        assert!(vec2_equals(there_and_back, point, 1e-9));
        assert!(mat3_equals(mat3_multiply(transform, undo), mat3_identity(), 1e-9));
    }

    #[test]
    fn mat3_inverse_refuses_a_flattening_transform() {
        assert!(mat3_inverse(mat3_scaling(0.0, 1.0)).is_err());
    }

    #[test]
    fn mat3_determinant_is_the_area_factor() {
        assert!(close(mat3_determinant(mat3_identity()), 1.0));
        assert!(close(mat3_determinant(mat3_scaling(2.0, 3.0)), 6.0));
        assert!(close(mat3_determinant(mat3_rotation(1.2)), 1.0));
    }

    #[test]
    fn mat3_transpose_swaps_rows_and_columns() {
        let matrix = mat3(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]).unwrap();
        assert_eq!(mat3_to_array(mat3_transpose(matrix)), vec![1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]);
    }

    #[test]
    fn mat3_get_checks_its_bounds() {
        let matrix = mat3(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]).unwrap();
        assert_eq!(mat3_get(matrix.clone(), 1, 2).unwrap(), 6.0);
        assert!(mat3_get(matrix.clone(), 3, 0).is_err());
        assert!(mat3_get(matrix, -1, 0).is_err());
    }
}
