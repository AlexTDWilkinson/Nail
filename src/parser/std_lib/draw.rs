//! Drawing pictures, with no window and no graphics card.
//!
//! Every function here returns a string. A shape is a string, a group of
//! shapes is a string, and a whole drawing is a string that happens to be an
//! SVG document - which browsers, editors, README files and print all
//! understand, and which `fs_write` saves like any other text.
//!
//! That is not a shortcut, it is the point. Because shapes are values, a
//! drawing is built the same way a Nail program builds everything else: map
//! over the data to get the shapes, join them, wrap them in `draw_svg`. There
//! is no canvas to mutate, no drawing context to keep hold of, and no order
//! dependence beyond the one that matters - shapes later in the array are
//! painted over shapes earlier in it.
//!
//! Coordinates start at the top left and y grows downward, which is the
//! convention every screen uses and the opposite of the one every graph uses.
//! To plot something, subtract from the height.

use serde::{Deserialize, Serialize};

/// Which part of a piece of text sits at the x it was placed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DRAW_Anchor {
    /// The text begins at x and runs right.
    Start,
    /// The text is centred on x - what a label over a bar wants.
    Middle,
    /// The text ends at x, for right-aligned labels.
    End,
}

/// XML-escapes text going into an attribute or a text node. Without this, a
/// label containing `<` or `&` silently produces a document nothing can open.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    return out;
}

/// Writes a number the way SVG wants it: no exponent, no trailing zeros, and
/// `0` rather than `-0`. Rust's default float formatting produces `1e-7` for
/// small numbers, which SVG readers are not required to accept.
fn number(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded == 0.0 {
        return "0".to_string();
    }
    let mut text = format!("{:.3}", rounded);
    while text.contains('.') && (text.ends_with('0') || text.ends_with('.')) {
        text.pop();
    }
    return text;
}

/// Wraps a finished set of shapes in an SVG document of the given size.
///
/// The size is in user units, which are the same units every coordinate below
/// is in, and which a browser shows as pixels unless told otherwise.
pub fn svg(width: f64, height: f64, background: String, shapes: Vec<String>) -> Result<String, String> {
    if width <= 0.0 || height <= 0.0 {
        return Err(format!("draw_svg: a drawing {} by {} has no area to draw in", number(width), number(height)));
    }

    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        number(width),
        number(height),
        number(width),
        number(height)
    );

    // An empty background means no background rectangle at all, which leaves
    // the drawing transparent - what you want when it is going on top of
    // something whose colour you do not know.
    if !background.is_empty() {
        out.push_str(&format!("\n  <rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"{}\"/>", number(width), number(height), escape(&background)));
    }

    for shape in shapes.iter() {
        out.push_str("\n  ");
        out.push_str(shape);
    }

    out.push_str("\n</svg>\n");
    return Ok(out);
}

/// A rectangle. `corner_radius` of 0 gives square corners.
pub fn rect(x: f64, y: f64, width: f64, height: f64, fill: String, corner_radius: f64) -> Result<String, String> {
    if width < 0.0 || height < 0.0 {
        return Err(format!("draw_rect: a rectangle {} by {} has a negative side", number(width), number(height)));
    }
    let corners = if corner_radius > 0.0 { format!(" rx=\"{}\"", number(corner_radius)) } else { String::new() };
    return Ok(format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"{}/>",
        number(x),
        number(y),
        number(width),
        number(height),
        escape(&fill),
        corners
    ));
}

/// A circle, given its centre and radius.
pub fn circle(center_x: f64, center_y: f64, radius: f64, fill: String) -> Result<String, String> {
    if radius < 0.0 {
        return Err(format!("draw_circle: a circle of radius {} cannot be drawn", number(radius)));
    }
    return Ok(format!("<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>", number(center_x), number(center_y), number(radius), escape(&fill)));
}

/// An ellipse, given its centre and its two radii.
pub fn ellipse(center_x: f64, center_y: f64, radius_x: f64, radius_y: f64, fill: String) -> Result<String, String> {
    if radius_x < 0.0 || radius_y < 0.0 {
        return Err(format!("draw_ellipse: an ellipse of radii {} and {} cannot be drawn", number(radius_x), number(radius_y)));
    }
    return Ok(format!(
        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\"/>",
        number(center_x),
        number(center_y),
        number(radius_x),
        number(radius_y),
        escape(&fill)
    ));
}

/// A straight line between two points.
pub fn line(x1: f64, y1: f64, x2: f64, y2: f64, stroke: String, stroke_width: f64) -> Result<String, String> {
    if stroke_width <= 0.0 {
        return Err(format!("draw_line: a line {} units wide would not be visible", number(stroke_width)));
    }
    return Ok(format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
        number(x1),
        number(y1),
        number(x2),
        number(y2),
        escape(&stroke),
        number(stroke_width)
    ));
}

/// Turns a flat array of x and y values into the `x,y x,y` SVG wants.
fn points_attribute(function: &str, points: &Vec<f64>) -> Result<String, String> {
    if points.len() % 2 != 0 {
        return Err(format!("{}: got {} numbers, and points come in pairs of x and y", function, points.len()));
    }
    if points.len() < 4 {
        return Err(format!("{}: got {} points, and at least two are needed", function, points.len() / 2));
    }

    let mut written = Vec::with_capacity(points.len() / 2);
    let mut index = 0;
    while index < points.len() {
        written.push(format!("{},{}", number(points[index]), number(points[index + 1])));
        index += 2;
    }
    return Ok(written.join(" "));
}

/// A run of connected line segments, given as a flat array of x and y values:
/// `[0.0, 0.0, 10.0, 5.0, 20.0, 2.0]` is three points. This is the shape a
/// line chart is made of.
pub fn polyline(points: Vec<f64>, stroke: String, stroke_width: f64) -> Result<String, String> {
    if stroke_width <= 0.0 {
        return Err(format!("draw_polyline: a line {} units wide would not be visible", number(stroke_width)));
    }
    let attribute = points_attribute("draw_polyline", &points)?;
    return Ok(format!("<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>", attribute, escape(&stroke), number(stroke_width)));
}

/// A closed shape through the given points, in the same flat form.
pub fn polygon(points: Vec<f64>, fill: String) -> Result<String, String> {
    let attribute = points_attribute("draw_polygon", &points)?;
    return Ok(format!("<polygon points=\"{}\" fill=\"{}\"/>", attribute, escape(&fill)));
}

/// Text at a point. The anchor says which part of the text sits at that x -
/// DRAW_Anchor::Middle is what a centred label wants.
pub fn text(x: f64, y: f64, content: String, size: f64, fill: String, anchor: DRAW_Anchor) -> Result<String, String> {
    if size <= 0.0 {
        return Err(format!("draw_text: text {} units tall would not be visible", number(size)));
    }
    let anchor = match anchor {
        DRAW_Anchor::Start => "start",
        DRAW_Anchor::Middle => "middle",
        DRAW_Anchor::End => "end",
    };
    return Ok(format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" fill=\"{}\" text-anchor=\"{}\">{}</text>",
        number(x),
        number(y),
        number(size),
        escape(&fill),
        anchor,
        escape(&content)
    ));
}

/// An arbitrary path, in SVG's own path notation: `M 0 0 L 10 10` and so on.
/// The escape hatch for a shape none of the others can make.
pub fn path(commands: String, stroke: String, stroke_width: f64, fill: String) -> Result<String, String> {
    if commands.trim().is_empty() {
        return Err("draw_path: the path has no commands in it".to_string());
    }
    let painted_fill = if fill.is_empty() { "none".to_string() } else { escape(&fill) };
    return Ok(format!(
        "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
        escape(&commands),
        painted_fill,
        escape(&stroke),
        number(stroke_width)
    ));
}

/// Several shapes moved together. The offset is applied to everything inside,
/// which is how a chart's plotting area is kept clear of its axis labels
/// without every coordinate inside having the margin added to it by hand.
pub fn group(offset_x: f64, offset_y: f64, shapes: Vec<String>) -> String {
    let mut out = format!("<g transform=\"translate({},{})\">", number(offset_x), number(offset_y));
    for shape in shapes.iter() {
        out.push_str("\n    ");
        out.push_str(shape);
    }
    out.push_str("\n  </g>");
    return out;
}

/// Moves a value from one range into another - the arithmetic every chart
/// needs and everyone gets subtly wrong. A data value between `from_low` and
/// `from_high` comes back as the matching position between `to_low` and
/// `to_high`, and to plot upward on a screen whose y grows downward you pass
/// the height as `to_low` and 0 as `to_high`.
pub fn scale(value: f64, from_low: f64, from_high: f64, to_low: f64, to_high: f64) -> Result<f64, String> {
    if from_low == from_high {
        return Err(format!("draw_scale: the range {} to {} is empty, so there is nowhere to place {} in it", number(from_low), number(from_high), number(value)));
    }
    let share = (value - from_low) / (from_high - from_low);
    return Ok(to_low + share * (to_high - to_low));
}

/// Where a point sits on a circle, with angles the way a clock reads them:
/// 0 degrees at twelve o'clock, growing clockwise. Screen y grows downward,
/// which is what makes clockwise come out clockwise.
fn point_on_circle(center_x: f64, center_y: f64, radius: f64, degrees: f64) -> (f64, f64) {
    let radians = (degrees - 90.0).to_radians();
    return (center_x + radius * radians.cos(), center_y + radius * radians.sin());
}

/// Checks a span of degrees is drawable: not empty, and less than the whole
/// circle, which `draw_circle` draws without the arc arithmetic.
fn usable_sweep(function: &str, start_degrees: f64, end_degrees: f64) -> Result<f64, String> {
    let sweep = end_degrees - start_degrees;
    if sweep == 0.0 {
        return Err(format!("{}: from {} to {} degrees spans nothing", function, number(start_degrees), number(end_degrees)));
    }
    if sweep.abs() >= 360.0 {
        return Err(format!("{}: {} degrees is the whole circle, which draw_circle draws directly", function, number(sweep.abs())));
    }
    return Ok(sweep);
}

/// The two flags an SVG `A` command needs: whether to take the long way round,
/// and which direction to sweep in. A positive sweep is clockwise.
fn arc_flags(sweep: f64) -> (u8, u8) {
    let large = if sweep.abs() > 180.0 { 1 } else { 0 };
    let direction = if sweep >= 0.0 { 1 } else { 0 };
    return (large, direction);
}

/// An arc stroke along part of a circle, between two angles read the way a
/// clock reads them: 0 degrees at twelve o'clock, growing clockwise. A gauge
/// is this arc twice - once faint for the track, once bright for the value.
pub fn arc(center_x: f64, center_y: f64, radius: f64, start_degrees: f64, end_degrees: f64, color: String, stroke_width: f64) -> Result<String, String> {
    if radius < 0.0 {
        return Err(format!("draw_arc: an arc of radius {} cannot be drawn", number(radius)));
    }
    if stroke_width <= 0.0 {
        return Err(format!("draw_arc: an arc {} units wide would not be visible", number(stroke_width)));
    }
    let sweep = usable_sweep("draw_arc", start_degrees, end_degrees)?;
    let (start_x, start_y) = point_on_circle(center_x, center_y, radius, start_degrees);
    let (end_x, end_y) = point_on_circle(center_x, center_y, radius, end_degrees);
    let (large, direction) = arc_flags(sweep);
    return Ok(format!(
        "<path d=\"M {} {} A {} {} 0 {} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
        number(start_x),
        number(start_y),
        number(radius),
        number(radius),
        large,
        direction,
        number(end_x),
        number(end_y),
        escape(&color),
        number(stroke_width)
    ));
}

/// A filled slice of a circle between two angles, in the same clockwise
/// degrees as `draw_arc` - the shape a pie chart is made of.
pub fn wedge(center_x: f64, center_y: f64, radius: f64, start_degrees: f64, end_degrees: f64, fill: String) -> Result<String, String> {
    if radius < 0.0 {
        return Err(format!("draw_wedge: a slice of radius {} cannot be drawn", number(radius)));
    }
    let sweep = usable_sweep("draw_wedge", start_degrees, end_degrees)?;
    let (start_x, start_y) = point_on_circle(center_x, center_y, radius, start_degrees);
    let (end_x, end_y) = point_on_circle(center_x, center_y, radius, end_degrees);
    let (large, direction) = arc_flags(sweep);
    return Ok(format!(
        "<path d=\"M {} {} L {} {} A {} {} 0 {} {} {} {} Z\" fill=\"{}\"/>",
        number(center_x),
        number(center_y),
        number(start_x),
        number(start_y),
        number(radius),
        number(radius),
        large,
        direction,
        number(end_x),
        number(end_y),
        escape(&fill)
    ));
}

/// A star of 3 to 24 points, its corners alternating between the outer and
/// inner radius, drawn point up.
pub fn star(center_x: f64, center_y: f64, points: i64, outer_radius: f64, inner_radius: f64, fill: String) -> Result<String, String> {
    if !(3..=24).contains(&points) {
        return Err(format!("draw_star: a star has between 3 and 24 points, got {}", points));
    }
    if outer_radius < 0.0 || inner_radius < 0.0 {
        return Err(format!("draw_star: a star of radii {} and {} cannot be drawn", number(outer_radius), number(inner_radius)));
    }
    let corner_count = points as usize * 2;
    let step = 360.0 / corner_count as f64;
    let mut corners: Vec<f64> = Vec::with_capacity(corner_count * 2);
    for corner in 0..corner_count {
        let radius = if corner % 2 == 0 { outer_radius } else { inner_radius };
        let (x, y) = point_on_circle(center_x, center_y, radius, corner as f64 * step);
        corners.push(x);
        corners.push(y);
    }
    let attribute = points_attribute("draw_star", &corners)?;
    return Ok(format!("<polygon points=\"{}\" fill=\"{}\"/>", attribute, escape(&fill)));
}

/// A regular polygon of 3 to 60 sides, every corner on one circle, drawn
/// point up.
pub fn regular_polygon(center_x: f64, center_y: f64, sides: i64, radius: f64, fill: String) -> Result<String, String> {
    if !(3..=60).contains(&sides) {
        return Err(format!("draw_regular_polygon: a polygon has between 3 and 60 sides, got {}", sides));
    }
    if radius < 0.0 {
        return Err(format!("draw_regular_polygon: a polygon of radius {} cannot be drawn", number(radius)));
    }
    let corner_count = sides as usize;
    let step = 360.0 / corner_count as f64;
    let mut corners: Vec<f64> = Vec::with_capacity(corner_count * 2);
    for corner in 0..corner_count {
        let (x, y) = point_on_circle(center_x, center_y, radius, corner as f64 * step);
        corners.push(x);
        corners.push(y);
    }
    let attribute = points_attribute("draw_regular_polygon", &corners)?;
    return Ok(format!("<polygon points=\"{}\" fill=\"{}\"/>", attribute, escape(&fill)));
}

/// A rectangle with rounded corners. The radius is clamped to half the
/// shorter side, so a generous radius makes a capsule rather than a mess.
pub fn rounded_rect(x: f64, y: f64, width: f64, height: f64, corner_radius: f64, fill: String) -> Result<String, String> {
    if width < 0.0 || height < 0.0 {
        return Err(format!("draw_rounded_rect: a rectangle {} by {} has a negative side", number(width), number(height)));
    }
    if corner_radius < 0.0 {
        return Err(format!("draw_rounded_rect: a corner of radius {} cannot be drawn", number(corner_radius)));
    }
    let clamped = corner_radius.min(width / 2.0).min(height / 2.0);
    return Ok(format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\"/>",
        number(x),
        number(y),
        number(width),
        number(height),
        number(clamped),
        number(clamped),
        escape(&fill)
    ));
}

/// A line with a filled head at its far end. The head is sized from the
/// stroke width, so a heavier arrow gets a proportionally bigger head.
pub fn arrow(from_x: f64, from_y: f64, to_x: f64, to_y: f64, color: String, stroke_width: f64) -> Result<String, String> {
    if stroke_width <= 0.0 {
        return Err(format!("draw_arrow: an arrow {} units wide would not be visible", number(stroke_width)));
    }
    let length = ((to_x - from_x).powi(2) + (to_y - from_y).powi(2)).sqrt();
    if length == 0.0 {
        return Err("draw_arrow: an arrow from a point to itself points nowhere".to_string());
    }
    let head_length = (stroke_width * 4.0).min(length);
    let head_half_width = stroke_width * 2.0;
    let unit_x = (to_x - from_x) / length;
    let unit_y = (to_y - from_y) / length;
    // The line stops at the base of the head, so the head has a clean point.
    let base_x = to_x - unit_x * head_length;
    let base_y = to_y - unit_y * head_length;
    let shaft = line(from_x, from_y, base_x, base_y, color.clone(), stroke_width)?;
    let head = polygon(
        vec![to_x, to_y, base_x - unit_y * head_half_width, base_y + unit_x * head_half_width, base_x + unit_y * head_half_width, base_y - unit_x * head_half_width],
        color,
    )?;
    return Ok(format!("{}{}", shaft, head));
}

/// Evenly spaced guide lines in both directions across the given area, for
/// laying a drawing out. Pass a light colour so the drawing stays on top.
pub fn grid(width: f64, height: f64, spacing: f64, color: String) -> Result<String, String> {
    if width <= 0.0 || height <= 0.0 {
        return Err(format!("draw_grid: a grid {} by {} has no area to draw in", number(width), number(height)));
    }
    if spacing <= 0.0 {
        return Err(format!("draw_grid: a grid needs a positive spacing, got {}", number(spacing)));
    }
    let line_count = (width / spacing).floor() + (height / spacing).floor() + 2.0;
    if line_count > 10_000.0 {
        return Err(format!("draw_grid: a spacing of {} across {} by {} would be {} lines, which is a texture rather than a grid", number(spacing), number(width), number(height), number(line_count)));
    }

    let mut out = String::new();
    let mut x = 0.0;
    while x <= width {
        out.push_str(&line(x, 0.0, x, height, color.clone(), 1.0)?);
        x += spacing;
    }
    let mut y = 0.0;
    while y <= height {
        out.push_str(&line(0.0, y, width, y, color.clone(), 1.0)?);
        y += spacing;
    }
    return Ok(out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_the_way_svg_accepts() {
        assert_eq!(number(1.0), "1");
        assert_eq!(number(1.5), "1.5");
        assert_eq!(number(1.23456), "1.235");
        assert_eq!(number(0.0), "0");
        assert_eq!(number(-0.0), "0");
        // Rust would write these as 1e-7 and inf, which SVG readers need not accept.
        assert_eq!(number(0.0000001), "0");
        assert_eq!(number(f64::INFINITY), "0");
        assert_eq!(number(f64::NAN), "0");
    }

    #[test]
    fn text_that_would_break_the_document_is_escaped() {
        let drawn = text(0.0, 0.0, "a < b & \"c\"".to_string(), 10.0, "black".to_string(), DRAW_Anchor::Start).expect("valid text");
        assert!(drawn.contains("a &lt; b &amp; &quot;c&quot;"), "got: {}", drawn);
        assert!(!drawn.contains("a < b"), "the raw text must not survive: {}", drawn);
    }

    #[test]
    fn a_document_wraps_its_shapes() {
        let shapes = vec![circle(50.0, 50.0, 10.0, "red".to_string()).expect("a valid circle")];
        let document = svg(100.0, 100.0, "white".to_string(), shapes).expect("a valid size");
        assert!(document.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(document.contains("viewBox=\"0 0 100 100\""));
        assert!(document.contains("<rect x=\"0\" y=\"0\" width=\"100\" height=\"100\" fill=\"white\"/>"));
        assert!(document.contains("<circle cx=\"50\" cy=\"50\" r=\"10\" fill=\"red\"/>"));
        assert!(document.ends_with("</svg>\n"));
    }

    #[test]
    fn an_empty_background_leaves_the_drawing_transparent() {
        let document = svg(10.0, 10.0, String::new(), vec![]).expect("a valid size");
        assert!(!document.contains("<rect"), "no background rectangle should be drawn: {}", document);
    }

    #[test]
    fn a_drawing_with_no_area_is_an_error() {
        assert!(svg(0.0, 10.0, String::new(), vec![]).unwrap_err().contains("no area"));
        assert!(svg(-1.0, 10.0, String::new(), vec![]).unwrap_err().contains("no area"));
    }

    #[test]
    fn shapes_render_as_themselves() {
        assert_eq!(rect(1.0, 2.0, 3.0, 4.0, "blue".to_string(), 0.0).expect("valid"), "<rect x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"blue\"/>");
        assert!(rect(1.0, 2.0, 3.0, 4.0, "blue".to_string(), 2.0).expect("valid").contains("rx=\"2\""));
        assert_eq!(line(0.0, 0.0, 1.0, 1.0, "black".to_string(), 2.0).expect("valid"), "<line x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\" stroke=\"black\" stroke-width=\"2\"/>");
        assert!(ellipse(0.0, 0.0, 2.0, 3.0, "green".to_string()).expect("valid").contains("rx=\"2\" ry=\"3\""));
    }

    #[test]
    fn shapes_that_could_not_be_seen_are_errors() {
        assert!(rect(0.0, 0.0, -1.0, 4.0, "blue".to_string(), 0.0).unwrap_err().contains("negative side"));
        assert!(circle(0.0, 0.0, -1.0, "blue".to_string()).unwrap_err().contains("cannot be drawn"));
        assert!(line(0.0, 0.0, 1.0, 1.0, "black".to_string(), 0.0).unwrap_err().contains("would not be visible"));
        assert!(text(0.0, 0.0, "hi".to_string(), 0.0, "black".to_string(), DRAW_Anchor::Start).unwrap_err().contains("would not be visible"));
        assert!(path(String::new(), "black".to_string(), 1.0, String::new()).unwrap_err().contains("no commands"));
    }

    #[test]
    fn each_anchor_lands_in_the_text_anchor_attribute() {
        for (anchor, spelled) in [(DRAW_Anchor::Start, "start"), (DRAW_Anchor::Middle, "middle"), (DRAW_Anchor::End, "end")] {
            let drawn = text(0.0, 0.0, "hi".to_string(), 10.0, "black".to_string(), anchor).expect("valid text");
            assert!(drawn.contains(&format!("text-anchor=\"{}\"", spelled)), "got: {}", drawn);
        }
    }

    #[test]
    fn points_come_in_pairs() {
        let drawn = polyline(vec![0.0, 0.0, 10.0, 5.0, 20.0, 2.0], "red".to_string(), 1.0).expect("valid points");
        assert!(drawn.contains("points=\"0,0 10,5 20,2\""), "got: {}", drawn);

        assert!(polyline(vec![0.0, 0.0, 10.0], "red".to_string(), 1.0).unwrap_err().contains("pairs of x and y"));
        assert!(polyline(vec![0.0, 0.0], "red".to_string(), 1.0).unwrap_err().contains("at least two are needed"));
        assert!(polygon(vec![0.0, 0.0, 1.0], "red".to_string()).unwrap_err().contains("pairs of x and y"));
    }

    #[test]
    fn a_group_moves_everything_inside_it() {
        let inner = vec![circle(0.0, 0.0, 1.0, "red".to_string()).expect("valid")];
        let grouped = group(10.0, 20.0, inner);
        assert!(grouped.starts_with("<g transform=\"translate(10,20)\">"));
        assert!(grouped.contains("<circle"));
        assert!(grouped.ends_with("</g>"));
    }

    #[test]
    fn scaling_moves_a_value_between_ranges() {
        assert_eq!(scale(5.0, 0.0, 10.0, 0.0, 100.0).expect("a real range"), 50.0);
        assert_eq!(scale(0.0, 0.0, 10.0, 0.0, 100.0).expect("a real range"), 0.0);
        assert_eq!(scale(10.0, 0.0, 10.0, 0.0, 100.0).expect("a real range"), 100.0);
        // Plotting upward on a screen whose y grows downward.
        assert_eq!(scale(10.0, 0.0, 10.0, 100.0, 0.0).expect("a real range"), 0.0);
        assert_eq!(scale(0.0, 0.0, 10.0, 100.0, 0.0).expect("a real range"), 100.0);
        assert!(scale(1.0, 5.0, 5.0, 0.0, 100.0).unwrap_err().contains("is empty"));
    }

    /// Pulls the vertices back out of a polygon's points attribute, so a test
    /// can check the geometry rather than the exact text.
    fn parsed_vertices(shape: &str) -> Vec<(f64, f64)> {
        let start = shape.find("points=\"").expect("a points attribute") + "points=\"".len();
        let end = shape[start..].find('"').expect("a closing quote") + start;
        return shape[start..end]
            .split(' ')
            .map(|pair| {
                let mut halves = pair.split(',');
                let x: f64 = halves.next().expect("an x").parse().expect("a number");
                let y: f64 = halves.next().expect("a y").parse().expect("a number");
                return (x, y);
            })
            .collect();
    }

    #[test]
    fn an_arc_is_a_stroked_path_with_an_arc_command() {
        let drawn = arc(100.0, 100.0, 80.0, 0.0, 270.0, "steelblue".to_string(), 8.0).expect("a valid arc");
        assert!(drawn.starts_with("<path"), "got: {}", drawn);
        assert!(drawn.contains(" A 80 80 "), "got: {}", drawn);
        assert!(drawn.contains("fill=\"none\""), "an arc is a stroke, not a fill: {}", drawn);
        assert!(drawn.contains("stroke=\"steelblue\""));
        assert!(drawn.contains("stroke-width=\"8\""));
        // 0 degrees is twelve o'clock, so the arc starts straight above the centre.
        assert!(drawn.contains("M 100 20"), "got: {}", drawn);
    }

    #[test]
    fn an_arc_that_spans_nothing_or_everything_is_an_error() {
        assert!(arc(0.0, 0.0, 10.0, 90.0, 90.0, "red".to_string(), 1.0).unwrap_err().contains("spans nothing"));
        assert!(arc(0.0, 0.0, 10.0, 0.0, 360.0, "red".to_string(), 1.0).unwrap_err().contains("whole circle"));
        assert!(arc(0.0, 0.0, -1.0, 0.0, 90.0, "red".to_string(), 1.0).unwrap_err().contains("cannot be drawn"));
        assert!(arc(0.0, 0.0, 10.0, 0.0, 90.0, "red".to_string(), 0.0).unwrap_err().contains("would not be visible"));
    }

    #[test]
    fn a_wedge_is_one_filled_path_through_an_arc() {
        let drawn = wedge(100.0, 100.0, 80.0, 0.0, 120.0, "gold".to_string()).expect("a valid wedge");
        assert_eq!(drawn.matches("<path").count(), 1, "got: {}", drawn);
        assert!(drawn.contains(" A 80 80 "), "got: {}", drawn);
        assert!(drawn.contains("fill=\"gold\""));
        assert!(drawn.contains("M 100 100 L"), "a wedge starts at the centre: {}", drawn);
        assert!(drawn.contains(" Z\""), "a wedge closes back to the centre: {}", drawn);
    }

    #[test]
    fn a_wedge_wider_than_half_the_circle_takes_the_long_way_round() {
        let drawn = wedge(0.0, 0.0, 10.0, 0.0, 270.0, "gold".to_string()).expect("a valid wedge");
        assert!(drawn.contains(" A 10 10 0 1 1 "), "the large arc flag must be set: {}", drawn);
        assert!(wedge(0.0, 0.0, 10.0, 45.0, 45.0, "gold".to_string()).unwrap_err().contains("spans nothing"));
        assert!(wedge(0.0, 0.0, 10.0, 0.0, 400.0, "gold".to_string()).unwrap_err().contains("whole circle"));
    }

    #[test]
    fn a_five_pointed_star_has_ten_vertices() {
        let drawn = star(50.0, 50.0, 5, 40.0, 16.0, "gold".to_string()).expect("a valid star");
        let vertices = parsed_vertices(&drawn);
        assert_eq!(vertices.len(), 10, "got: {}", drawn);
        assert!(drawn.contains("fill=\"gold\""));
        // The corners alternate between the two radii, starting at the outer one.
        for (index, (x, y)) in vertices.iter().enumerate() {
            let distance = ((x - 50.0).powi(2) + (y - 50.0).powi(2)).sqrt();
            let expected = if index % 2 == 0 { 40.0 } else { 16.0 };
            assert!((distance - expected).abs() < 0.01, "vertex {} sits {} from the centre, wanted {}", index, distance, expected);
        }
    }

    #[test]
    fn a_two_pointed_star_is_an_error() {
        assert!(star(0.0, 0.0, 2, 10.0, 4.0, "gold".to_string()).unwrap_err().contains("between 3 and 24"));
        assert!(star(0.0, 0.0, 25, 10.0, 4.0, "gold".to_string()).unwrap_err().contains("between 3 and 24"));
        assert!(star(0.0, 0.0, 5, -1.0, 4.0, "gold".to_string()).unwrap_err().contains("cannot be drawn"));
    }

    #[test]
    fn a_regular_polygon_keeps_every_corner_on_its_circle() {
        let drawn = regular_polygon(0.0, 0.0, 4, 10.0, "green".to_string()).expect("a valid polygon");
        let vertices = parsed_vertices(&drawn);
        assert_eq!(vertices.len(), 4);
        for (x, y) in vertices {
            let distance = (x.powi(2) + y.powi(2)).sqrt();
            assert!((distance - 10.0).abs() < 0.01, "a corner sits {} from the centre, wanted 10", distance);
        }
        assert!(regular_polygon(0.0, 0.0, 2, 10.0, "green".to_string()).unwrap_err().contains("between 3 and 60"));
        assert!(regular_polygon(0.0, 0.0, 61, 10.0, "green".to_string()).unwrap_err().contains("between 3 and 60"));
    }

    #[test]
    fn a_rounded_rectangle_clamps_its_radius_to_the_shorter_side() {
        let drawn = rounded_rect(0.0, 0.0, 20.0, 10.0, 3.0, "blue".to_string()).expect("a valid rectangle");
        assert!(drawn.contains("rx=\"3\" ry=\"3\""), "got: {}", drawn);
        let capsule = rounded_rect(0.0, 0.0, 20.0, 10.0, 50.0, "blue".to_string()).expect("a valid rectangle");
        assert!(capsule.contains("rx=\"5\" ry=\"5\""), "the radius clamps to half the shorter side: {}", capsule);
        assert!(rounded_rect(0.0, 0.0, -1.0, 10.0, 2.0, "blue".to_string()).unwrap_err().contains("negative side"));
        assert!(rounded_rect(0.0, 0.0, 20.0, 10.0, -2.0, "blue".to_string()).unwrap_err().contains("cannot be drawn"));
    }

    #[test]
    fn an_arrow_is_a_line_with_a_filled_head() {
        let drawn = arrow(0.0, 0.0, 100.0, 0.0, "black".to_string(), 2.0).expect("a valid arrow");
        assert!(drawn.contains("<line"), "got: {}", drawn);
        assert!(drawn.contains("<polygon"), "got: {}", drawn);
        assert!(drawn.contains("stroke=\"black\""));
        assert!(drawn.contains("fill=\"black\""));
        // The head's tip is the far end, and the shaft stops at the head's base.
        assert!(drawn.contains("points=\"100,0"), "the tip sits at the far end: {}", drawn);
        assert!(drawn.contains("x2=\"92\""), "the shaft stops at the base of the head: {}", drawn);
    }

    #[test]
    fn an_arrow_going_nowhere_is_an_error() {
        assert!(arrow(5.0, 5.0, 5.0, 5.0, "black".to_string(), 2.0).unwrap_err().contains("points nowhere"));
        assert!(arrow(0.0, 0.0, 10.0, 0.0, "black".to_string(), 0.0).unwrap_err().contains("would not be visible"));
    }

    #[test]
    fn a_grid_rules_lines_in_both_directions() {
        let drawn = grid(100.0, 50.0, 25.0, "#e2e8f0".to_string()).expect("a valid grid");
        // Vertical lines at 0, 25, 50, 75 and 100, horizontal at 0, 25 and 50.
        assert_eq!(drawn.matches("<line").count(), 8, "got: {}", drawn);
        assert!(drawn.contains("x1=\"75\" y1=\"0\" x2=\"75\" y2=\"50\""), "got: {}", drawn);
        assert!(drawn.contains("x1=\"0\" y1=\"25\" x2=\"100\" y2=\"25\""), "got: {}", drawn);
        assert!(drawn.contains("stroke=\"#e2e8f0\""));
    }

    #[test]
    fn a_grid_that_could_not_be_drawn_is_an_error() {
        assert!(grid(0.0, 50.0, 10.0, "grey".to_string()).unwrap_err().contains("no area"));
        assert!(grid(100.0, 50.0, 0.0, "grey".to_string()).unwrap_err().contains("positive spacing"));
        assert!(grid(10000.0, 10000.0, 0.1, "grey".to_string()).unwrap_err().contains("texture"));
    }

    /// The whole point of shapes being values: a chart is a map and a join,
    /// with no canvas and no drawing order to keep track of.
    #[test]
    fn a_chart_is_built_by_mapping_over_data() {
        let data = vec![3.0, 7.0, 5.0, 9.0, 2.0];
        let mut points: Vec<f64> = Vec::new();
        for (index, value) in data.iter().enumerate() {
            points.push(scale(index as f64, 0.0, (data.len() - 1) as f64, 0.0, 200.0).expect("a real range"));
            points.push(scale(*value, 0.0, 10.0, 100.0, 0.0).expect("a real range"));
        }

        let shapes = vec![polyline(points, "steelblue".to_string(), 2.0).expect("valid points")];
        let document = svg(200.0, 100.0, "white".to_string(), shapes).expect("a valid size");
        assert!(document.contains("<polyline"));
        assert!(document.contains("0,70"), "the first point plots upward: {}", document);
    }
}

/// A QR code of the text as an SVG document, black on white, sized to scale
/// cleanly. Put a URL in it and a phone camera opens the page.
pub fn qr_svg(text: String) -> Result<String, String> {
    if text.is_empty() {
        return Err("draw_qr_svg: there is nothing to encode in an empty string".to_string());
    }
    let code = qrcode::QrCode::new(text.as_bytes()).map_err(|e| format!("draw_qr_svg: could not build the code: {}", e))?;
    let document = code.render::<qrcode::render::svg::Color>().min_dimensions(240, 240).build();
    return Ok(document);
}

#[cfg(test)]
mod qr_tests {
    use super::qr_svg;

    #[test]
    fn a_url_becomes_an_svg_document() {
        let document = qr_svg("https://nail-lang.org".to_string()).unwrap();
        assert!(document.starts_with("<?xml"));
        assert!(document.contains("<svg"));
        assert!(qr_svg("".to_string()).unwrap_err().contains("nothing to encode"));
    }
}
