//! Statistics module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Stats:
        "stats_mean" => "std_lib::stats::mean", (values: (&[f])) -> (f!e),
            "Returns the average of the values; errors on an empty array.",
            "average:f = danger(stats_mean(prices));";
        "stats_median" => "std_lib::stats::median", (values: (&[f])) -> (f!e),
            "Returns the middle value once sorted, which one outlier cannot move; errors on an empty array.",
            "middle:f = danger(stats_median(prices));";
        "stats_mode" => "std_lib::stats::mode", (values: (&[f])) -> (f!e),
            "Returns the value that appears most often, the smallest one when several tie; errors on an empty array.",
            "most_common:f = danger(stats_mode(ratings));";
        "stats_variance" => "std_lib::stats::variance", (values: (&[f])) -> (f!e),
            "Returns the sample variance; errors on fewer than two values.",
            "spread:f = danger(stats_variance(prices));";
        "stats_stddev" => "std_lib::stats::stddev", (values: (&[f])) -> (f!e),
            "Returns the sample standard deviation, in the units of the data; errors on fewer than two values.",
            "spread:f = danger(stats_stddev(prices));";
        "stats_percentile" => "std_lib::stats::percentile", (values: (&[f]), share: f) -> (f!e),
            "Returns the value below which the given share of the data falls, written from 0.0 to 1.0.",
            "p95:f = danger(stats_percentile(latencies, 0.95));";
        "stats_range" => "std_lib::stats::range", (values: (&[f])) -> (f!e),
            "Returns the distance from the smallest value to the largest; errors on an empty array.",
            "span:f = danger(stats_range(prices));";
        "stats_correlation" => "std_lib::stats::correlation", (first: (&[f]), second: (&[f])) -> (f!e),
            "Returns how closely two columns move together, from -1.0 to 1.0; errors on mismatched lengths or a column that never changes.",
            "link:f = danger(stats_correlation(spend, revenue));";
    }
}
