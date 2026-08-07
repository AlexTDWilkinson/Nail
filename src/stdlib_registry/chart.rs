//! Chart module stdlib registry entries.
//!
//! Each function returns a whole SVG document, so a chart is a string like any
//! other - written to a file, or put straight into a page.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Chart:
        "chart_line" => "std_lib::chart::line", (width: f, height: f, values: [f], labels: [s], colour: s, title: s) -> (s!e),
            "Returns an SVG line chart of evenly spaced values. Labels are placed under the points at their own index, so fewer labels than points is fine.",
            "daily_totals:a:f = [820.0, 910.0, 1180.0];\nday_names:a:s = [`Mon`, `Tue`, `Wed`];\nsvg:s = danger(chart_line(600.0, 300.0, daily_totals, day_names, `#2563eb`, `Requests`));";
        "chart_bar" => "std_lib::chart::bar", (width: f, height: f, values: [f], labels: [s], colour: s, title: s) -> (s!e),
            "Returns an SVG bar chart, one bar per value, with the axis always including zero so the bars are comparable.",
            "sales:a:f = [1200.0, 950.0, 1730.0];\nregions:a:s = [`West`, `Prairies`, `East`];\nsvg:s = danger(chart_bar(600.0, 300.0, sales, regions, `#16a34a`, `Sales`));";
        "chart_scatter" => "std_lib::chart::scatter", (width: f, height: f, x_values: [f], y_values: [f], colour: s, title: s) -> (s!e),
            "Returns an SVG scatter plot, reading the two arrays together so the first x goes with the first y.",
            "predicted:a:f = [11.0, 29.0, 42.0];\nactual:a:f = [10.0, 30.0, 40.0];\nsvg:s = danger(chart_scatter(400.0, 400.0, predicted, actual, `#dc2626`, `Fit`));";
        "chart_sparkline" => "std_lib::chart::sparkline", (width: f, height: f, values: [f], colour: s) -> (s!e),
            "Returns a small SVG line with no axis, labels or background, for putting a shape beside a number in a table or a line of prose.",
            "last_week:a:f = [3.0, 5.0, 4.0, 8.0, 6.0, 9.0, 7.0];\nsvg:s = danger(chart_sparkline(80.0, 20.0, last_week, `#2563eb`));";
        "chart_pie" => "std_lib::chart::pie", (labels: [s], values: [f]) -> (s!e),
            "Returns an SVG pie chart of shares of a whole, one slice per value with a legend of names and percentages, colours dealt from a fixed palette in order.",
            "regions:a:s = [`West`, `Prairies`, `East`];\nsales:a:f = [1200.0, 950.0, 1730.0];\nsvg:s = danger(chart_pie(regions, sales));";
        "chart_donut" => "std_lib::chart::donut", (labels: [s], values: [f]) -> (s!e),
            "Returns an SVG donut chart - a pie with a hole - with the total written in the middle.",
            "sources:a:s = [`search`, `direct`, `social`];\nvisits:a:f = [4200.0, 3100.0, 900.0];\nsvg:s = danger(chart_donut(sources, visits));";
        "chart_histogram" => "std_lib::chart::histogram", (values: [f], bins: i) -> (s!e),
            "Returns an SVG histogram of the values in 1 to 100 equal width bins, drawn as touching bars with the bin edges written along the x axis.",
            "response_times:a:f = [12.0, 18.0, 15.0, 44.0, 13.0, 21.0];\nsvg:s = danger(chart_histogram(response_times, 10));";
    }
}
