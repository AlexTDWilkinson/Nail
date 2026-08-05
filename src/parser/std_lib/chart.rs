//! Charts, as complete SVG documents.
//!
//! The `draw_*` functions are enough to plot anything, and plotting anything
//! comes to the same fifty lines every time: work out the range, leave room for
//! the labels, scale each value into the plot area, remember that y grows
//! downward. That is written here once for the three charts a program actually
//! reaches for - a line over time, bars by category, and a scatter of pairs.
//!
//! Each function returns a whole SVG document, so a chart is a value like any
//! other string: written to a file with `fs_write`, or put straight in a page,
//! since an inline `<svg>` needs no separate request and no image file.
//!
//! Anything more particular than these - a second axis, a legend, stacked bars -
//! is built from `draw_*` directly. These are the common cases, not a
//! replacement for drawing.

use crate::parser::std_lib::draw;

/// How much room the labels need around the plot area. Fixed rather than
/// measured, because measuring text means knowing the font.
const LEFT_MARGIN: f64 = 56.0;
const RIGHT_MARGIN: f64 = 16.0;
const TOP_MARGIN: f64 = 32.0;
const BOTTOM_MARGIN: f64 = 40.0;

const AXIS_COLOUR: &str = "#94a3b8";
const GRID_COLOUR: &str = "#e2e8f0";
const LABEL_COLOUR: &str = "#475569";
const TITLE_COLOUR: &str = "#1e293b";

/// A value written for an axis label: whole numbers with no decimal point,
/// anything else to a sensible number of places rather than seventeen.
fn tick_label(value: f64) -> String {
    if (value - value.round()).abs() < 0.000_001 && value.abs() < 1_000_000_000.0 {
        return format!("{}", value.round() as i64);
    }
    let magnitude = value.abs();
    let places = if magnitude >= 100.0 {
        1
    } else if magnitude >= 1.0 {
        2
    } else {
        3
    };
    let text = format!("{:.*}", places, value);
    return text.trim_end_matches('0').trim_end_matches('.').to_string();
}

/// The range to plot, widened so the highest value is not drawn on the frame
/// and so a set of identical values still has a plot area.
fn plot_range(lowest: f64, highest: f64) -> (f64, f64) {
    if (highest - lowest).abs() < f64::EPSILON {
        // Every value the same: centre it rather than dividing by a zero range.
        let value = highest;
        let padding = if value.abs() < f64::EPSILON { 1.0 } else { value.abs() * 0.1 };
        return (value - padding, value + padding);
    }
    return (lowest, highest);
}

/// Refuses a value that cannot be plotted. Checked directly rather than by
/// looking at the range afterwards, because `f64::min` and `f64::max` skip a
/// NaN rather than propagating it - a chart of `[1.0, NaN]` would otherwise
/// look like a chart of `[1.0, 1.0]`.
fn all_finite(values: &[f64], function_name: &str) -> Result<(), String> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("{}: the values include something that is not a number", function_name));
    }
    return Ok(());
}

fn size_is_usable(width: f64, height: f64, function_name: &str) -> Result<(), String> {
    let plot_width = width - LEFT_MARGIN - RIGHT_MARGIN;
    let plot_height = height - TOP_MARGIN - BOTTOM_MARGIN;
    if plot_width < 20.0 || plot_height < 20.0 {
        return Err(format!(
            "{}: a chart {} by {} leaves no room to plot in - the labels need about {} across and {} down",
            function_name,
            tick_label(width),
            tick_label(height),
            tick_label(LEFT_MARGIN + RIGHT_MARGIN + 20.0),
            tick_label(TOP_MARGIN + BOTTOM_MARGIN + 20.0)
        ));
    }
    return Ok(());
}

/// The frame, the horizontal grid lines and the value labels down the left.
/// Five lines including both ends, which is enough to read a value off and few
/// enough to stay legible at any size a chart is likely to be.
fn value_axis(width: f64, height: f64, lowest: f64, highest: f64, shapes: &mut Vec<String>) -> Result<(), String> {
    let plot_left = LEFT_MARGIN;
    let plot_right = width - RIGHT_MARGIN;
    let plot_top = TOP_MARGIN;
    let plot_bottom = height - BOTTOM_MARGIN;

    for step in 0..5 {
        let share = step as f64 / 4.0;
        let y = plot_bottom - share * (plot_bottom - plot_top);
        let value = lowest + share * (highest - lowest);
        // The bottom line is the axis itself, so it is drawn darker.
        let colour = if step == 0 { AXIS_COLOUR } else { GRID_COLOUR };
        shapes.push(draw::line(plot_left, y, plot_right, y, colour.to_string(), 1.0)?);
        shapes.push(draw::text(plot_left - 8.0, y + 4.0, tick_label(value), 11.0, LABEL_COLOUR.to_string(), "end".to_string())?);
    }
    shapes.push(draw::line(plot_left, plot_top, plot_left, plot_bottom, AXIS_COLOUR.to_string(), 1.0)?);
    return Ok(());
}

fn title_shape(width: f64, title: String) -> Result<Vec<String>, String> {
    if title.is_empty() {
        return Ok(vec![]);
    }
    return Ok(vec![draw::text(width / 2.0, TOP_MARGIN - 12.0, title, 14.0, TITLE_COLOUR.to_string(), "middle".to_string())?]);
}

/// A line chart of evenly spaced values, with the labels written under the
/// points they belong to. Fewer labels than points is fine - every label is
/// placed under the point at its own index, and the rest go unlabelled - which
/// is how a chart of a hundred days shows a dozen dates.
pub fn line(width: f64, height: f64, values: Vec<f64>, labels: Vec<String>, colour: String, title: String) -> Result<String, String> {
    size_is_usable(width, height, "chart_line")?;
    if values.len() < 2 {
        return Err(format!("chart_line: a line needs at least two values, got {}", values.len()));
    }

    all_finite(&values, "chart_line")?;
    let lowest = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let highest = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let (low, high) = plot_range(lowest, highest);

    let plot_left = LEFT_MARGIN;
    let plot_right = width - RIGHT_MARGIN;
    let plot_top = TOP_MARGIN;
    let plot_bottom = height - BOTTOM_MARGIN;

    let mut shapes = title_shape(width, title)?;
    value_axis(width, height, low, high, &mut shapes)?;

    let horizontal_step = (plot_right - plot_left) / (values.len() - 1) as f64;
    let mut points: Vec<f64> = Vec::with_capacity(values.len() * 2);
    for (index, value) in values.iter().enumerate() {
        let x = plot_left + index as f64 * horizontal_step;
        // y grows downward, so a larger value is a smaller y.
        let y = draw::scale(*value, low, high, plot_bottom, plot_top)?;
        points.push(x);
        points.push(y);
    }
    shapes.push(draw::polyline(points.clone(), colour.clone(), 2.0)?);

    // Points are drawn on top of the line, so a single reading is visible even
    // where the line is flat.
    for pair in points.chunks(2) {
        shapes.push(draw::circle(pair[0], pair[1], 2.5, colour.clone())?);
    }

    for (index, label) in labels.iter().enumerate() {
        if index >= values.len() || label.is_empty() {
            continue;
        }
        let x = plot_left + index as f64 * horizontal_step;
        shapes.push(draw::text(x, plot_bottom + 18.0, label.clone(), 11.0, LABEL_COLOUR.to_string(), "middle".to_string())?);
    }

    return draw::svg(width, height, "#ffffff".to_string(), shapes);
}

/// A bar chart, one bar per value, labelled underneath. Bars start at zero
/// rather than at the lowest value, because a bar's length is what is being
/// compared and a bar that does not start at zero compares nothing.
pub fn bar(width: f64, height: f64, values: Vec<f64>, labels: Vec<String>, colour: String, title: String) -> Result<String, String> {
    size_is_usable(width, height, "chart_bar")?;
    if values.is_empty() {
        return Err("chart_bar: there were no values to draw".to_string());
    }

    all_finite(&values, "chart_bar")?;
    let highest = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let lowest = values.iter().cloned().fold(f64::INFINITY, f64::min);
    // Zero is always in range, so the bars are comparable.
    let (low, high) = plot_range(lowest.min(0.0), highest.max(0.0));

    let plot_left = LEFT_MARGIN;
    let plot_right = width - RIGHT_MARGIN;
    let plot_top = TOP_MARGIN;
    let plot_bottom = height - BOTTOM_MARGIN;

    let mut shapes = title_shape(width, title)?;
    value_axis(width, height, low, high, &mut shapes)?;

    let slot = (plot_right - plot_left) / values.len() as f64;
    let bar_width = (slot * 0.7).max(1.0);
    let zero_y = draw::scale(0.0, low, high, plot_bottom, plot_top)?;

    for (index, value) in values.iter().enumerate() {
        let slot_left = plot_left + index as f64 * slot;
        let bar_left = slot_left + (slot - bar_width) / 2.0;
        let value_y = draw::scale(*value, low, high, plot_bottom, plot_top)?;
        // A negative value hangs below the zero line rather than above it.
        let top = value_y.min(zero_y);
        let bar_height = (value_y - zero_y).abs().max(1.0);
        shapes.push(draw::rect(bar_left, top, bar_width, bar_height, colour.clone(), 2.0)?);

        if let Some(label) = labels.get(index) {
            if !label.is_empty() {
                shapes.push(draw::text(slot_left + slot / 2.0, plot_bottom + 18.0, label.clone(), 11.0, LABEL_COLOUR.to_string(), "middle".to_string())?);
            }
        }
    }

    return draw::svg(width, height, "#ffffff".to_string(), shapes);
}

/// A scatter of pairs: the two arrays are read together, so the first x goes
/// with the first y. Both axes are scaled to their own values, since the point
/// of a scatter is the shape of the cloud rather than either value's size.
pub fn scatter(width: f64, height: f64, x_values: Vec<f64>, y_values: Vec<f64>, colour: String, title: String) -> Result<String, String> {
    size_is_usable(width, height, "chart_scatter")?;
    if x_values.len() != y_values.len() {
        return Err(format!("chart_scatter: there are {} x values and {} y values, and each point needs one of each", x_values.len(), y_values.len()));
    }
    if x_values.is_empty() {
        return Err("chart_scatter: there were no points to draw".to_string());
    }

    all_finite(&x_values, "chart_scatter")?;
    all_finite(&y_values, "chart_scatter")?;
    let x_lowest = x_values.iter().cloned().fold(f64::INFINITY, f64::min);
    let x_highest = x_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let y_lowest = y_values.iter().cloned().fold(f64::INFINITY, f64::min);
    let y_highest = y_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let (x_low, x_high) = plot_range(x_lowest, x_highest);
    let (y_low, y_high) = plot_range(y_lowest, y_highest);

    let plot_left = LEFT_MARGIN;
    let plot_right = width - RIGHT_MARGIN;
    let plot_top = TOP_MARGIN;
    let plot_bottom = height - BOTTOM_MARGIN;

    let mut shapes = title_shape(width, title)?;
    value_axis(width, height, y_low, y_high, &mut shapes)?;

    // The x axis gets its own labels at the ends and the middle, which is as
    // much as fits without knowing how wide the numbers are.
    for step in 0..3 {
        let share = step as f64 / 2.0;
        let x = plot_left + share * (plot_right - plot_left);
        let value = x_low + share * (x_high - x_low);
        let anchor = match step {
            0 => "start",
            2 => "end",
            _ => "middle",
        };
        shapes.push(draw::text(x, plot_bottom + 18.0, tick_label(value), 11.0, LABEL_COLOUR.to_string(), anchor.to_string())?);
    }

    for (x_value, y_value) in x_values.iter().zip(y_values.iter()) {
        let x = draw::scale(*x_value, x_low, x_high, plot_left, plot_right)?;
        let y = draw::scale(*y_value, y_low, y_high, plot_bottom, plot_top)?;
        shapes.push(draw::circle(x, y, 3.0, colour.clone())?);
    }

    return draw::svg(width, height, "#ffffff".to_string(), shapes);
}

/// A row of small bars with no axis, labels or margins, for putting a shape
/// next to a number in a table or a line of prose. The chart equivalent of a
/// word rather than a paragraph.
pub fn sparkline(width: f64, height: f64, values: Vec<f64>, colour: String) -> Result<String, String> {
    if width <= 0.0 || height <= 0.0 {
        return Err(format!("chart_sparkline: a sparkline {} by {} has no area to draw in", tick_label(width), tick_label(height)));
    }
    if values.len() < 2 {
        return Err(format!("chart_sparkline: a sparkline needs at least two values, got {}", values.len()));
    }
    all_finite(&values, "chart_sparkline")?;
    let lowest = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let highest = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let (low, high) = plot_range(lowest, highest);

    let step = width / (values.len() - 1) as f64;
    let mut points: Vec<f64> = Vec::with_capacity(values.len() * 2);
    for (index, value) in values.iter().enumerate() {
        points.push(index as f64 * step);
        points.push(draw::scale(*value, low, high, height - 1.0, 1.0)?);
    }
    // No background: a sparkline sits on whatever colour the surrounding text
    // is on.
    return draw::svg(width, height, String::new(), vec![draw::polyline(points, colour, 1.5)?]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(values: &[&str]) -> Vec<String> {
        return values.iter().map(|value| value.to_string()).collect();
    }

    #[test]
    fn a_line_chart_is_a_whole_svg_document() {
        let chart = line(600.0, 300.0, vec![1.0, 5.0, 3.0, 8.0], labels(&["mon", "tue", "wed", "thu"]), "#2563eb".to_string(), "Requests".to_string()).expect("a drawable chart");
        assert!(chart.starts_with("<svg"), "got: {}", &chart[..40.min(chart.len())]);
        assert!(chart.trim_end().ends_with("</svg>"));
        assert!(chart.contains("polyline"));
        assert!(chart.contains("Requests"));
        assert!(chart.contains("mon"));
        assert!(chart.contains("thu"));
    }

    #[test]
    fn the_axis_is_labelled_with_the_range_of_the_values() {
        let chart = line(600.0, 300.0, vec![0.0, 100.0], labels(&[]), "#000".to_string(), String::new()).expect("a drawable chart");
        assert!(chart.contains(">0<"), "got: {}", chart);
        assert!(chart.contains(">100<"), "got: {}", chart);
        assert!(chart.contains(">50<"), "got: {}", chart);
    }

    #[test]
    fn fewer_labels_than_points_labels_the_points_it_has() {
        let chart = line(600.0, 300.0, vec![1.0, 2.0, 3.0, 4.0], labels(&["first"]), "#000".to_string(), String::new()).expect("a drawable chart");
        assert!(chart.contains("first"));
    }

    #[test]
    fn a_line_needs_two_points_to_be_a_line() {
        assert!(line(600.0, 300.0, vec![1.0], labels(&[]), "#000".to_string(), String::new()).is_err());
        assert!(line(600.0, 300.0, vec![], labels(&[]), "#000".to_string(), String::new()).is_err());
    }

    #[test]
    fn values_that_are_all_the_same_still_draw() {
        let chart = line(600.0, 300.0, vec![7.0, 7.0, 7.0], labels(&[]), "#000".to_string(), String::new()).expect("a drawable chart");
        assert!(chart.contains("polyline"));
        let flat_bars = bar(600.0, 300.0, vec![0.0, 0.0], labels(&[]), "#000".to_string(), String::new()).expect("a drawable chart");
        assert!(flat_bars.contains("rect"));
    }

    #[test]
    fn a_bar_chart_has_one_bar_for_each_value() {
        let chart = bar(600.0, 300.0, vec![3.0, 6.0, 9.0], labels(&["a", "b", "c"]), "#16a34a".to_string(), "Sales".to_string()).expect("a drawable chart");
        // One rect per bar, plus the document background.
        assert_eq!(chart.matches("<rect").count(), 4, "got: {}", chart);
        assert!(chart.contains("Sales"));
    }

    #[test]
    fn a_negative_bar_hangs_below_the_zero_line() {
        let chart = bar(600.0, 300.0, vec![10.0, -10.0], labels(&[]), "#000".to_string(), String::new()).expect("a drawable chart");
        assert_eq!(chart.matches("<rect").count(), 3);
        // Zero is in range whichever way the values go.
        assert!(chart.contains(">0<"), "got: {}", chart);
    }

    #[test]
    fn a_scatter_reads_the_two_arrays_together() {
        let chart = scatter(400.0, 400.0, vec![1.0, 2.0, 3.0], vec![10.0, 20.0, 15.0], "#dc2626".to_string(), "Fit".to_string()).expect("a drawable chart");
        assert_eq!(chart.matches("<circle").count(), 3);
        assert!(chart.contains("Fit"));
    }

    #[test]
    fn a_scatter_needs_a_y_for_every_x() {
        let failure = scatter(400.0, 400.0, vec![1.0, 2.0], vec![1.0], "#000".to_string(), String::new()).unwrap_err();
        assert!(failure.contains("each point needs one of each"), "got: {}", failure);
        assert!(scatter(400.0, 400.0, vec![], vec![], "#000".to_string(), String::new()).is_err());
    }

    #[test]
    fn a_sparkline_has_no_axis_and_no_background() {
        let spark = sparkline(80.0, 20.0, vec![1.0, 3.0, 2.0, 5.0], "#000".to_string()).expect("a drawable sparkline");
        assert!(spark.contains("polyline"));
        assert!(!spark.contains("<rect"), "got: {}", spark);
        assert!(!spark.contains("<text"), "got: {}", spark);
    }

    #[test]
    fn a_chart_too_small_to_label_says_so_rather_than_drawing_nonsense() {
        let failure = line(40.0, 40.0, vec![1.0, 2.0], labels(&[]), "#000".to_string(), String::new()).unwrap_err();
        assert!(failure.contains("no room to plot in"), "got: {}", failure);
        assert!(bar(40.0, 40.0, vec![1.0], labels(&[]), "#000".to_string(), String::new()).is_err());
    }

    #[test]
    fn a_value_that_is_not_a_number_is_refused() {
        assert!(line(600.0, 300.0, vec![1.0, f64::NAN], labels(&[]), "#000".to_string(), String::new()).is_err());
        assert!(bar(600.0, 300.0, vec![f64::INFINITY], labels(&[]), "#000".to_string(), String::new()).is_err());
    }

    #[test]
    fn a_label_that_is_markup_cannot_break_the_document() {
        let chart = bar(600.0, 300.0, vec![1.0, 2.0], labels(&["<script>", "a & b"]), "#000".to_string(), "<title>".to_string()).expect("a drawable chart");
        assert!(!chart.contains("<script>"), "got: {}", chart);
        assert!(chart.contains("&lt;script&gt;"));
        assert!(chart.contains("a &amp; b"));
    }
}
