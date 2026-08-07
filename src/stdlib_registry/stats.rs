//! Statistics module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Stats:
        "stats_mean" => "std_lib::stats::mean", (values: (&[f])) -> (f!e),
            "Returns the average of the values. Errors on an empty array.",
            "average:f = danger(stats_mean(prices));";
        "stats_median" => "std_lib::stats::median", (values: (&[f])) -> (f!e),
            "Returns the middle value once sorted, which one outlier cannot move. Errors on an empty array.",
            "middle:f = danger(stats_median(prices));";
        "stats_mode" => "std_lib::stats::mode", (values: (&[f])) -> (f!e),
            "Returns the value that appears most often, the smallest one when several tie. Errors on an empty array.",
            "most_common:f = danger(stats_mode(ratings));";
        "stats_variance" => "std_lib::stats::variance", (values: (&[f])) -> (f!e),
            "Returns the sample variance. Errors on fewer than two values.",
            "spread:f = danger(stats_variance(prices));";
        "stats_stddev" => "std_lib::stats::stddev", (values: (&[f])) -> (f!e),
            "Returns the sample standard deviation, in the units of the data. Errors on fewer than two values.",
            "spread:f = danger(stats_stddev(prices));";
        "stats_percentile" => "std_lib::stats::percentile", (values: (&[f]), share: f) -> (f!e),
            "Returns the value below which the given share of the data falls, written from 0.0 to 1.0.",
            "p95:f = danger(stats_percentile(latencies, 0.95));";
        "stats_range" => "std_lib::stats::range", (values: (&[f])) -> (f!e),
            "Returns the distance from the smallest value to the largest. Errors on an empty array.",
            "span:f = danger(stats_range(prices));";
        "stats_correlation" => "std_lib::stats::correlation", (first: (&[f]), second: (&[f])) -> (f!e),
            "Returns how closely two columns move together, from -1.0 to 1.0. Errors on mismatched lengths or a column that never changes.",
            "link:f = danger(stats_correlation(spend, revenue));";
        "stats_geometric_mean" => "std_lib::stats::geometric_mean", (values: (&[f])) -> (f!e),
            "Returns the n-th root of the product, the right average for growth rates and ratios. Errors unless every value is positive.",
            "typical_growth:f = danger(stats_geometric_mean(growth_factors));";
        "stats_harmonic_mean" => "std_lib::stats::harmonic_mean", (values: (&[f])) -> (f!e),
            "Returns the reciprocal of the mean of reciprocals, the right average for rates like speeds. Errors unless every value is positive.",
            "average_speed:f = danger(stats_harmonic_mean(speeds));";
        "stats_weighted_mean" => "std_lib::stats::weighted_mean", (values: (&[f]), weights: (&[f])) -> (f!e),
            "Returns the mean with each value counted by its weight. Errors on mismatched lengths or weights that do not sum to a positive total.",
            "grade:f = danger(stats_weighted_mean(scores, credits));";
        "stats_trimmed_mean" => "std_lib::stats::trimmed_mean", (values: (&[f]), trim_share: f) -> (f!e),
            "Returns the mean after dropping the given share of values from each end, blunting outliers. The share runs from 0.0 up to but not including 0.5.",
            "steady:f = danger(stats_trimmed_mean(latencies, 0.1));";
        "stats_iqr" => "std_lib::stats::iqr", (values: (&[f])) -> (f!e),
            "Returns the width of the middle half of the data, a spread one outlier cannot inflate. Errors on an empty array.",
            "middle_spread:f = danger(stats_iqr(latencies));";
        "stats_mad" => "std_lib::stats::mad", (values: (&[f])) -> (f!e),
            "Returns the median distance from the median, the most outlier-resistant spread measure. Errors on an empty array.",
            "spread:f = danger(stats_mad(prices));";
        "stats_skewness" => "std_lib::stats::skewness", (values: (&[f])) -> (f!e),
            "Returns adjusted sample skewness, positive when the long tail points right. Errors on fewer than three values or flat data.",
            "lean:f = danger(stats_skewness(incomes));";
        "stats_kurtosis" => "std_lib::stats::kurtosis", (values: (&[f])) -> (f!e),
            "Returns excess sample kurtosis, positive for heavy tails and negative for flat-topped data. Errors on fewer than four values or flat data.",
            "tails:f = danger(stats_kurtosis(returns));";
        "stats_sem" => "std_lib::stats::sem", (values: (&[f])) -> (f!e),
            "Returns the standard error of the mean, how far the sample mean likely sits from the true one. Errors on fewer than two values.",
            "wobble:f = danger(stats_sem(measurements));";
        "stats_cv" => "std_lib::stats::cv", (values: (&[f])) -> (f!e),
            "Returns the standard deviation as a share of the mean, comparable across different units. Errors on fewer than two values or a zero mean.",
            "relative_spread:f = danger(stats_cv(prices));";
        "stats_pvariance" => "std_lib::stats::pvariance", (values: (&[f])) -> (f!e),
            "Returns the population variance, dividing by n, for when the values are the whole population. Errors on an empty array.",
            "spread:f = danger(stats_pvariance(census_ages));";
        "stats_pstddev" => "std_lib::stats::pstddev", (values: (&[f])) -> (f!e),
            "Returns the population standard deviation, in the units of the data. Errors on an empty array.",
            "spread:f = danger(stats_pstddev(census_ages));";
        "stats_rms" => "std_lib::stats::rms", (values: (&[f])) -> (f!e),
            "Returns the root mean square, the natural magnitude for values that swing through zero. Errors on an empty array.",
            "magnitude:f = danger(stats_rms(errors));";
        "stats_midrange" => "std_lib::stats::midrange", (values: (&[f])) -> (f!e),
            "Returns the midpoint between the smallest and largest value. Errors on an empty array.",
            "center:f = danger(stats_midrange(temperatures));";
        "stats_covariance" => "std_lib::stats::covariance", (first: (&[f]), second: (&[f])) -> (f!e),
            "Returns the sample covariance, positive when two columns rise together, in the product of their units. Errors on mismatched lengths or fewer than two pairs.",
            "together:f = danger(stats_covariance(spend, revenue));";
        "stats_rank" => "std_lib::stats::rank", (values: (&[f])) -> [f],
            "Returns the 1-based rank of each value in the original order, tied values sharing the average of their positions.",
            "positions:a:f = stats_rank(scores);";
        "stats_spearman" => "std_lib::stats::spearman", (first: (&[f]), second: (&[f])) -> (f!e),
            "Returns rank correlation, which sees any steadily rising or falling relationship, straight line or not. Errors on mismatched lengths, fewer than two pairs, or a flat column.",
            "link:f = danger(stats_spearman(spend, revenue));";
        "stats_zscores" => "std_lib::stats::zscores", (values: (&[f])) -> ([f]!e),
            "Returns each value as its distance from the mean in standard deviations. Errors on fewer than two values or flat data.",
            "scores:a:f = danger(stats_zscores(measurements));";
        "stats_normalize" => "std_lib::stats::normalize", (values: (&[f])) -> ([f]!e),
            "Returns each value scaled linearly onto 0.0..1.0, smallest to largest. Errors on an empty array or flat data.",
            "scaled:a:f = danger(stats_normalize(prices));";
        "stats_cumulative_sum" => "std_lib::stats::cumulative_sum", (values: (&[f])) -> [f],
            "Returns the running total after each value. An empty array stays empty.",
            "totals:a:f = stats_cumulative_sum(daily_sales);";
        "stats_differences" => "std_lib::stats::differences", (values: (&[f])) -> [f],
            "Returns the step from each value to the next, one shorter than the input. An empty array stays empty.",
            "steps:a:f = stats_differences(temperatures);";
        "stats_percent_change" => "std_lib::stats::percent_change", (values: (&[f])) -> ([f]!e),
            "Returns the percent change from each value to the next, one shorter than the input. Errors when a step starts from zero.",
            "growth:a:f = danger(stats_percent_change(monthly_revenue));";
        "stats_moving_average" => "std_lib::stats::moving_average", (values: (&[f]), window: i) -> ([f]!e),
            "Returns the mean of each window-sized run of neighbours, smoothing a noisy series. Errors unless the window fits inside the array.",
            "smooth:a:f = danger(stats_moving_average(prices, 7));";
        "stats_ewma" => "std_lib::stats::ewma", (values: (&[f]), alpha: f) -> ([f]!e),
            "Returns the exponentially weighted moving average - the factor is above 0.0 and at most 1.0, smaller meaning smoother. Errors on an empty array or a factor outside that range.",
            "trend:a:f = danger(stats_ewma(prices, 0.3));";
        "stats_histogram" => "std_lib::stats::histogram", (values: (&[f]), bins: i) -> ([i]!e),
            "Returns counts per equal-width bin from the smallest value to the largest, the largest landing in the last bin. Errors on an empty array or fewer than one bin.",
            "counts:a:i = danger(stats_histogram(latencies, 10));";
        "stats_outliers" => "std_lib::stats::outliers", (values: (&[f])) -> [f],
            "Returns the values beyond the 1.5-IQR boxplot fences, in their original order. Fewer than four values report none.",
            "unusual:a:f = stats_outliers(latencies);";
        "stats_percentile_rank" => "std_lib::stats::percentile_rank", (values: (&[f]), target: f) -> (f!e),
            "Returns the share of values at or below the target, from 0.0 to 100.0 - the inverse question to stats_percentile. Errors on an empty array.",
            "standing:f = danger(stats_percentile_rank(scores, 88.0));";
        "stats_quartiles" => "std_lib::stats::quartiles", (values: (&[f])) -> ([f]!e),
            "Returns the 25th, 50th and 75th percentiles as a three-value array, the box of a boxplot in one call. Errors on an empty array.",
            "box:a:f = danger(stats_quartiles(latencies));";
        "stats_normal_cdf" => "std_lib::stats::normal_cdf", (value: f, mean: f, stddev: f) -> (f!e),
            "Returns the probability that a normal draw lands at or below the value. Errors unless the standard deviation is positive.",
            "share:f = danger(stats_normal_cdf(1.96, 0.0, 1.0));";
        "stats_normal_inverse" => "std_lib::stats::normal_inverse", (probability: f, mean: f, stddev: f) -> (f!e),
            "Returns the value below which the given share of a normal distribution falls, the inverse of stats_normal_cdf. Errors unless the probability is strictly between 0 and 1 and the standard deviation is positive.",
            "cutoff:f = danger(stats_normal_inverse(0.975, 0.0, 1.0));";
        "stats_normal_pdf" => "std_lib::stats::normal_pdf", (value: f, mean: f, stddev: f) -> (f!e),
            "Returns the height of the normal bell curve at the value, a density rather than a probability. Errors unless the standard deviation is positive.",
            "height:f = danger(stats_normal_pdf(0.0, 0.0, 1.0));";
        "stats_binomial_pmf" => "std_lib::stats::binomial_pmf", (successes: i, trials: i, probability: f) -> (f!e),
            "Returns the probability of exactly that many successes in the trials, computed in log space so a thousand trials cannot overflow. Errors unless the successes run from 0 to the trials and the probability from 0.0 to 1.0.",
            "chance:f = danger(stats_binomial_pmf(5, 10, 0.5));";
        "stats_binomial_cdf" => "std_lib::stats::binomial_cdf", (successes: i, trials: i, probability: f) -> (f!e),
            "Returns the probability of at most that many successes in the trials, the pmf summed in log space. Errors unless the successes run from 0 to the trials and the probability from 0.0 to 1.0.",
            "chance:f = danger(stats_binomial_cdf(5, 10, 0.5));";
        "stats_poisson_pmf" => "std_lib::stats::poisson_pmf", (events: i, rate: f) -> (f!e),
            "Returns the probability of exactly that many events arriving at the given average rate, computed in log space so large counts cannot overflow. Errors unless the rate is positive and the count nonnegative.",
            "chance:f = danger(stats_poisson_pmf(2, 3.0));";
        "stats_poisson_cdf" => "std_lib::stats::poisson_cdf", (events: i, rate: f) -> (f!e),
            "Returns the probability of at most that many events arriving at the given average rate, the pmf summed in log space. Errors unless the rate is positive and the count nonnegative.",
            "chance:f = danger(stats_poisson_cdf(2, 3.0));";
        "stats_sample_size_for_proportion" => "std_lib::stats::sample_size_for_proportion", (margin_of_error: f, confidence: f) -> (i!e),
            "Returns how many people to survey so an estimated proportion lands within the margin of error at the given confidence, the classic poll planning number, rounded up. Errors unless both the margin and the confidence are strictly between 0 and 1.",
            "respondents:i = danger(stats_sample_size_for_proportion(0.03, 0.95));";
        "stats_t_test" => "std_lib::stats::t_test", (first: (&[f]), second: (&[f])) -> (f!e),
            "Returns the two-sided p-value of a Welch two-sample t-test, the chance of a gap this large between the means if the groups truly matched. Errors unless each sample has at least two values and at least one sample has spread.",
            "p_value:f = danger(stats_t_test(control_times, variant_times));";
        "stats_chi_square_test" => "std_lib::stats::chi_square_test", (observed: (&[f]), expected: (&[f])) -> (f!e),
            "Returns the goodness-of-fit p-value comparing observed counts against expected ones, the chance of a mismatch this large if the expectation were right. Errors on mismatched lengths, fewer than two cells, or an expected count that is not positive.",
            "p_value:f = danger(stats_chi_square_test(observed_counts, expected_counts));";
        "stats_proportion_test" => "std_lib::stats::proportion_test", (successes_a: i, total_a: i, successes_b: i, total_b: i) -> (f!e),
            "Returns the two-sided p-value of a pooled two-proportion z-test, the chance of a gap this large between two success rates if they truly matched. Errors unless each total is at least one, each success count fits its total, and the outcomes vary at all.",
            "p_value:f = danger(stats_proportion_test(200, 1000, 250, 1000));";
        "stats_ab_test" => "std_lib::stats::ab_test", (conversions_a: i, visitors_a: i, conversions_b: i, visitors_b: i) -> (f!e),
            "Returns the p-value that variant B converts differently from variant A, the two-proportion z-test in experiment words, with 0.05 the conventional bar for calling a winner. Errors unless each arm has at least one visitor and the conversions fit their visitors.",
            "p_value:f = danger(stats_ab_test(120, 2400, 156, 2400));";
        "stats_confidence_interval_95" => "std_lib::stats::confidence_interval_95", (values: (&[f])) -> (f!e),
            "Returns the plus-or-minus half width of the 95 percent t-interval for the mean, the distance the true mean sits within 95 percent of the time. Errors on fewer than two values.",
            "margin:f = danger(stats_confidence_interval_95(measurements));";
        "stats_min_detectable_effect" => "std_lib::stats::min_detectable_effect", (visitors_per_arm: i, baseline_rate: f) -> (f!e),
            "Returns the smallest absolute rate change an A/B test with that many visitors per arm can reliably detect, at 80 percent power and two-sided 5 percent significance. Errors unless each arm has at least one visitor and the baseline rate sits strictly between 0 and 1.",
            "effect:f = danger(stats_min_detectable_effect(5000, 0.042));";
    }
}
