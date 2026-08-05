//! Drawing module stdlib registry entries.
//!
//! Every function returns a string, so a drawing composes the way everything
//! else in Nail composes - map over the data to get shapes, join them, wrap
//! them in draw_svg. No canvas, no drawing context, no dependencies.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Draw:
        "draw_svg" => "std_lib::draw::svg", (width: f, height: f, background: s, shapes: [s]) -> (s!e),
            "Wraps shapes in an SVG document of the given size. An empty background leaves the drawing transparent. Save it with fs_write.",
            "picture:s = danger(draw_svg(400.0, 300.0, `white`, shapes));";
        "draw_rect" => "std_lib::draw::rect", (x: f, y: f, width: f, height: f, fill: s, corner_radius: f) -> (s!e),
            "A rectangle. A corner radius of 0.0 gives square corners.",
            "bar:s = danger(draw_rect(10.0, 20.0, 30.0, 40.0, `steelblue`, 0.0));";
        "draw_circle" => "std_lib::draw::circle", (center_x: f, center_y: f, radius: f, fill: s) -> (s!e),
            "A circle, given its centre and radius.",
            "dot:s = danger(draw_circle(50.0, 50.0, 4.0, `crimson`));";
        "draw_ellipse" => "std_lib::draw::ellipse", (center_x: f, center_y: f, radius_x: f, radius_y: f, fill: s) -> (s!e),
            "An ellipse, given its centre and its two radii.",
            "oval:s = danger(draw_ellipse(50.0, 50.0, 20.0, 10.0, `gold`));";
        "draw_line" => "std_lib::draw::line", (x1: f, y1: f, x2: f, y2: f, stroke: s, stroke_width: f) -> (s!e),
            "A straight line between two points.",
            "axis:s = danger(draw_line(0.0, 100.0, 200.0, 100.0, `black`, 1.0));";
        "draw_polyline" => "std_lib::draw::polyline", (points: [f], stroke: s, stroke_width: f) -> (s!e),
            "A run of connected line segments, given as a flat array of x and y values. This is the shape a line chart is made of.",
            "series:s = danger(draw_polyline(points, `steelblue`, 2.0));";
        "draw_polygon" => "std_lib::draw::polygon", (points: [f], fill: s) -> (s!e),
            "A closed shape through the given points, in the same flat array of x and y values.",
            "area:s = danger(draw_polygon(points, `lightblue`));";
        "draw_text" => "std_lib::draw::text", (x: f, y: f, content: s, size: f, fill: s, anchor: s) -> (s!e),
            "Text at a point. The anchor is start, middle or end, and says which part of the text sits at that x.",
            "label:s = danger(draw_text(100.0, 20.0, `Revenue`, 14.0, `black`, `middle`));";
        "draw_path" => "std_lib::draw::path", (commands: s, stroke: s, stroke_width: f, fill: s) -> (s!e),
            "An arbitrary path in SVG's own path notation - the escape hatch for a shape none of the others can make. An empty fill leaves it unfilled.",
            "shape:s = danger(draw_path(`M 0 0 L 10 10`, `black`, 1.0, ``));";
        "draw_group" => "std_lib::draw::group", (offset_x: f, offset_y: f, shapes: [s]) -> s,
            "Several shapes moved together, which is how a chart's plotting area is kept clear of its labels without adding the margin to every coordinate by hand.",
            "plot:s = draw_group(40.0, 20.0, shapes);";
        "draw_scale" => "std_lib::draw::scale", (value: f, from_low: f, from_high: f, to_low: f, to_high: f) -> (f!e),
            "Moves a value from one range into another - the arithmetic every chart needs. To plot upward on a screen whose y grows downward, pass the height as to_low and 0.0 as to_high.",
            "y:f = danger(draw_scale(value, 0.0, 100.0, 300.0, 0.0));";
        "draw_qr_svg" [QrCode] => "std_lib::draw::qr_svg", (text: s) -> (s!e),
            "A QR code of the text as an SVG document, black on white. Put a URL in it and a phone camera opens the page - tickets, table menus, 2FA enrolment.",
            "badge:s = danger(draw_qr_svg(`https://nail-lang.org`));";
    }
}
