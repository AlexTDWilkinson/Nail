//! Summary statistics over an array of numbers.
//!
//! The handful of questions anyone asks of a column of data: what is the
//! typical value, how spread out is it, where does the ninety-fifth percentile
//! sit, do these two columns move together. Every one of them is undefined on
//! an empty array, so every one returns a result rather than a number invented
//! out of nothing.

use super::math::erfc;

/// Shared guard, so an empty array fails the same way everywhere.
fn require_values(function: &str, values: &Vec<f64>) -> Result<(), String> {
    if values.is_empty() {
        return Err(format!("{}: the array is empty, so there is nothing to summarize", function));
    }
    return Ok(());
}

/// Smallest and largest value in one pass, for the summaries that need both
/// ends. Callers guard against an empty array before asking.
fn bounds(values: &Vec<f64>) -> (f64, f64) {
    let mut low = values[0];
    let mut high = values[0];
    for value in values.iter() {
        if *value < low {
            low = *value;
        }
        if *value > high {
            high = *value;
        }
    }
    return (low, high);
}

/// Sorted copy, so every summary orders NaN-free data the same way.
fn sorted_copy(values: &Vec<f64>) -> Vec<f64> {
    let mut sorted = values.clone();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    return sorted;
}

/// The arithmetic mean - the sum divided by the count.
pub fn mean(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_mean", values)?;
    let total: f64 = values.iter().sum();
    return Ok(total / values.len() as f64);
}

/// The middle value once sorted, or the average of the middle two when the
/// count is even. Unlike the mean, one absurd outlier does not move it.
pub fn median(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_median", values)?;
    let sorted = sorted_copy(values);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        return Ok((sorted[middle - 1] + sorted[middle]) / 2.0);
    }
    return Ok(sorted[middle]);
}

/// The sample variance, dividing by n-1 rather than n. That is the estimate to
/// use when the numbers are a sample of something larger, which they almost
/// always are; it needs at least two values to mean anything.
pub fn variance(values: &Vec<f64>) -> Result<f64, String> {
    if values.len() < 2 {
        return Err(format!("stats_variance: needs at least two values to measure spread, got {}", values.len()));
    }
    let average = mean(values)?;
    let total: f64 = values.iter().map(|value| (value - average).powi(2)).sum();
    return Ok(total / (values.len() - 1) as f64);
}

/// The square root of the variance, back in the units of the data.
pub fn stddev(values: &Vec<f64>) -> Result<f64, String> {
    let spread = variance(values).map_err(|_| format!("stats_stddev: needs at least two values to measure spread, got {}", values.len()))?;
    return Ok(spread.sqrt());
}

/// The value below which the given share of the data falls, with the share
/// written from 0.0 to 1.0 - so 0.5 is the median and 0.95 the ninety-fifth
/// percentile. Between two data points the answer is interpolated rather than
/// rounded to a neighbour.
pub fn percentile(values: &Vec<f64>, share: f64) -> Result<f64, String> {
    require_values("stats_percentile", values)?;
    if !(0.0..=1.0).contains(&share) {
        return Err(format!("stats_percentile: {} is not a share between 0.0 and 1.0", share));
    }

    let sorted = sorted_copy(values);
    if sorted.len() == 1 {
        return Ok(sorted[0]);
    }

    let position = share * (sorted.len() - 1) as f64;
    let below = position.floor() as usize;
    let above = position.ceil() as usize;
    if below == above {
        return Ok(sorted[below]);
    }
    let weight = position - below as f64;
    return Ok(sorted[below] * (1.0 - weight) + sorted[above] * weight);
}

/// The distance from the smallest value to the largest.
pub fn range(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_range", values)?;
    let mut low = values[0];
    let mut high = values[0];
    for value in values.iter() {
        if *value < low {
            low = *value;
        }
        if *value > high {
            high = *value;
        }
    }
    return Ok(high - low);
}

/// Pearson correlation: 1.0 when the two columns rise together in lockstep,
/// -1.0 when one falls as the other rises, 0.0 when knowing one tells you
/// nothing about the other. The arrays must be the same length, and neither
/// may be flat - a column that never changes has no correlation with anything.
pub fn correlation(first: &Vec<f64>, second: &Vec<f64>) -> Result<f64, String> {
    if first.len() != second.len() {
        return Err(format!("stats_correlation: the arrays have {} and {} values, and must be the same length", first.len(), second.len()));
    }
    if first.len() < 2 {
        return Err(format!("stats_correlation: needs at least two pairs of values, got {}", first.len()));
    }

    let first_mean = mean(first)?;
    let second_mean = mean(second)?;

    let mut covariance = 0.0;
    let mut first_spread = 0.0;
    let mut second_spread = 0.0;
    for index in 0..first.len() {
        let first_delta = first[index] - first_mean;
        let second_delta = second[index] - second_mean;
        covariance += first_delta * second_delta;
        first_spread += first_delta * first_delta;
        second_spread += second_delta * second_delta;
    }

    if first_spread == 0.0 || second_spread == 0.0 {
        return Err("stats_correlation: one of the arrays holds the same value throughout, so it correlates with nothing".to_string());
    }
    return Ok(covariance / (first_spread.sqrt() * second_spread.sqrt()));
}

/// The value that appears most often. When several values tie, the smallest of
/// them is returned, so the answer is the same on every run. Comparison is on
/// the exact number, which is the right thing for the counted, rounded data
/// anybody asks this of.
pub fn mode(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_mode", values)?;

    let sorted = sorted_copy(values);

    let mut best = sorted[0];
    let mut best_count = 0;
    let mut current = sorted[0];
    let mut current_count = 0;
    for value in sorted.iter() {
        if *value == current {
            current_count += 1;
        } else {
            current = *value;
            current_count = 1;
        }
        // Strictly greater, so the first value to reach a count keeps it - and
        // since the array is sorted, that is the smallest of any tie.
        if current_count > best_count {
            best = current;
            best_count = current_count;
        }
    }
    return Ok(best);
}

/// The n-th root of the product, computed in logs so a long array cannot
/// overflow on the way there. The right average for growth rates and ratios,
/// and only defined when every value is positive.
pub fn geometric_mean(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_geometric_mean", values)?;
    let mut log_total = 0.0;
    for value in values.iter() {
        if *value <= 0.0 {
            return Err(format!("stats_geometric_mean: {} is not positive, and the geometric mean is only defined for positive values", value));
        }
        log_total += value.ln();
    }
    return Ok((log_total / values.len() as f64).exp());
}

/// The reciprocal of the mean of reciprocals. The right average for rates -
/// speed over equal distances, price per unit - and only defined for positive
/// values.
pub fn harmonic_mean(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_harmonic_mean", values)?;
    let mut reciprocal_total = 0.0;
    for value in values.iter() {
        if *value <= 0.0 {
            return Err(format!("stats_harmonic_mean: {} is not positive, and the harmonic mean is only defined for positive values", value));
        }
        reciprocal_total += 1.0 / value;
    }
    return Ok(values.len() as f64 / reciprocal_total);
}

/// The mean with each value counted in proportion to its weight. The arrays
/// must pair up one to one, and the weights must add up to something positive
/// or there is nothing to share the average out by.
pub fn weighted_mean(values: &Vec<f64>, weights: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_weighted_mean", values)?;
    if values.len() != weights.len() {
        return Err(format!("stats_weighted_mean: there are {} values but {} weights, and every value needs one", values.len(), weights.len()));
    }
    let weight_total: f64 = weights.iter().sum();
    if weight_total <= 0.0 {
        return Err(format!("stats_weighted_mean: the weights sum to {}, and only a positive total can share out an average", weight_total));
    }
    let weighted_total: f64 = values.iter().zip(weights.iter()).map(|(value, weight)| value * weight).sum();
    return Ok(weighted_total / weight_total);
}

/// The mean after dropping floor(n * share) values from each end of the sorted
/// data, so a few wild readings cannot drag it. The share runs from 0.0 up to
/// but not including 0.5, since trimming half from both ends leaves nothing.
pub fn trimmed_mean(values: &Vec<f64>, trim_share: f64) -> Result<f64, String> {
    require_values("stats_trimmed_mean", values)?;
    if !(0.0..0.5).contains(&trim_share) {
        return Err(format!("stats_trimmed_mean: {} is not a trim share from 0.0 up to but not including 0.5", trim_share));
    }
    let sorted = sorted_copy(values);
    let dropped = (sorted.len() as f64 * trim_share).floor() as usize;
    if dropped * 2 >= sorted.len() {
        return Err(format!("stats_trimmed_mean: trimming {} values from each end of {} leaves nothing to average", dropped, sorted.len()));
    }
    let kept = &sorted[dropped..sorted.len() - dropped];
    return Ok(kept.iter().sum::<f64>() / kept.len() as f64);
}

/// The interquartile range - the width of the middle half of the data. A
/// spread measure one outlier cannot inflate the way it inflates the range.
pub fn iqr(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_iqr", values)?;
    let lower = percentile(values, 0.25)?;
    let upper = percentile(values, 0.75)?;
    return Ok(upper - lower);
}

/// The median absolute deviation - the median distance from the median. The
/// most outlier-resistant spread measure here; a third of the data can go bad
/// before it moves.
pub fn mad(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_mad", values)?;
    let center = median(values)?;
    let deviations: Vec<f64> = values.iter().map(|value| (value - center).abs()).collect();
    return median(&deviations);
}

/// Adjusted sample skewness: 0.0 for symmetric data, positive when the long
/// tail points right, negative when it points left. Needs at least three
/// values, and flat data has no asymmetry to measure.
pub fn skewness(values: &Vec<f64>) -> Result<f64, String> {
    if values.len() < 3 {
        return Err(format!("stats_skewness: needs at least three values to measure asymmetry, got {}", values.len()));
    }
    let average = mean(values)?;
    let spread = stddev(values)?;
    if spread == 0.0 {
        return Err("stats_skewness: the array holds the same value throughout, so it has no asymmetry to measure".to_string());
    }
    let count = values.len() as f64;
    let total: f64 = values.iter().map(|value| ((value - average) / spread).powi(3)).sum();
    return Ok(total * count / ((count - 1.0) * (count - 2.0)));
}

/// Excess sample kurtosis: 0.0 for a normal bell curve, positive for heavy
/// tails, negative for flat-topped data. Needs at least four values, and flat
/// data has no tails to weigh.
pub fn kurtosis(values: &Vec<f64>) -> Result<f64, String> {
    if values.len() < 4 {
        return Err(format!("stats_kurtosis: needs at least four values to measure tails, got {}", values.len()));
    }
    let average = mean(values)?;
    let spread = stddev(values)?;
    if spread == 0.0 {
        return Err("stats_kurtosis: the array holds the same value throughout, so it has no tails to measure".to_string());
    }
    let count = values.len() as f64;
    let total: f64 = values.iter().map(|value| ((value - average) / spread).powi(4)).sum();
    let heavy = total * count * (count + 1.0) / ((count - 1.0) * (count - 2.0) * (count - 3.0));
    let normal = 3.0 * (count - 1.0).powi(2) / ((count - 2.0) * (count - 3.0));
    return Ok(heavy - normal);
}

/// The standard error of the mean - how far the sample mean itself is likely
/// to sit from the true one. Shrinks with the square root of the count.
pub fn sem(values: &Vec<f64>) -> Result<f64, String> {
    let spread = stddev(values).map_err(|_| format!("stats_sem: needs at least two values to measure spread, got {}", values.len()))?;
    return Ok(spread / (values.len() as f64).sqrt());
}

/// The coefficient of variation - the standard deviation as a share of the
/// mean, so columns in different units can be compared. Undefined when the
/// mean is zero, since spread cannot be a share of nothing.
pub fn cv(values: &Vec<f64>) -> Result<f64, String> {
    let spread = stddev(values).map_err(|_| format!("stats_cv: needs at least two values to measure spread, got {}", values.len()))?;
    let average = mean(values)?;
    if average == 0.0 {
        return Err("stats_cv: the mean is zero, so spread cannot be expressed as a share of it".to_string());
    }
    return Ok(spread / average);
}

/// The population variance, dividing by n rather than n-1. The estimate to use
/// when the values are the whole population rather than a sample of one; a
/// single value has a variance of zero.
pub fn pvariance(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_pvariance", values)?;
    let average = mean(values)?;
    let total: f64 = values.iter().map(|value| (value - average).powi(2)).sum();
    return Ok(total / values.len() as f64);
}

/// The square root of the population variance, back in the units of the data.
pub fn pstddev(values: &Vec<f64>) -> Result<f64, String> {
    let spread = pvariance(values).map_err(|_| "stats_pstddev: the array is empty, so there is nothing to summarize".to_string())?;
    return Ok(spread.sqrt());
}

/// The root mean square - the square root of the mean of the squares. The
/// natural magnitude for values that swing through zero, like signals and
/// errors, where the plain mean would cancel itself out.
pub fn rms(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_rms", values)?;
    let total: f64 = values.iter().map(|value| value * value).sum();
    return Ok((total / values.len() as f64).sqrt());
}

/// The midpoint between the smallest and largest value. Cheap to compute, but
/// two outliers own it completely.
pub fn midrange(values: &Vec<f64>) -> Result<f64, String> {
    require_values("stats_midrange", values)?;
    let (low, high) = bounds(values);
    return Ok((low + high) / 2.0);
}

/// The sample covariance, dividing by n-1: positive when two columns rise
/// together, negative when one falls as the other rises, in the product of
/// their units. Correlation is this, rescaled to -1.0..1.0.
pub fn covariance(first: &Vec<f64>, second: &Vec<f64>) -> Result<f64, String> {
    if first.len() != second.len() {
        return Err(format!("stats_covariance: the arrays have {} and {} values, and must be the same length", first.len(), second.len()));
    }
    if first.len() < 2 {
        return Err(format!("stats_covariance: needs at least two pairs of values, got {}", first.len()));
    }
    let first_mean = mean(first)?;
    let second_mean = mean(second)?;
    let total: f64 = first.iter().zip(second.iter()).map(|(left, right)| (left - first_mean) * (right - second_mean)).sum();
    return Ok(total / (first.len() - 1) as f64);
}

/// The 1-based rank of each value, reported in the original order, with tied
/// values sharing the average of the positions they occupy - so ranks always
/// sum to the same total no matter how many ties there are.
pub fn rank(values: &Vec<f64>) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&left, &right| values[left].partial_cmp(&values[right]).unwrap_or(std::cmp::Ordering::Equal));

    let mut ranks = vec![0.0; values.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start;
        while end + 1 < order.len() && values[order[end + 1]] == values[order[start]] {
            end += 1;
        }
        // Positions start..=end hold one tied run; each member gets their average.
        let shared = (start + end) as f64 / 2.0 + 1.0;
        for position in start..=end {
            ranks[order[position]] = shared;
        }
        start = end + 1;
    }
    return ranks;
}

/// Spearman rank correlation: Pearson correlation on the ranks rather than the
/// values, so it sees any steadily rising or falling relationship, straight
/// line or not, and outliers only count for their position.
pub fn spearman(first: &Vec<f64>, second: &Vec<f64>) -> Result<f64, String> {
    if first.len() != second.len() {
        return Err(format!("stats_spearman: the arrays have {} and {} values, and must be the same length", first.len(), second.len()));
    }
    if first.len() < 2 {
        return Err(format!("stats_spearman: needs at least two pairs of values, got {}", first.len()));
    }
    let first_ranks = rank(first);
    let second_ranks = rank(second);
    // With the lengths already checked, correlation can only fail on a flat column.
    return correlation(&first_ranks, &second_ranks)
        .map_err(|_| "stats_spearman: one of the arrays holds the same value throughout, so its ranks correlate with nothing".to_string());
}

/// Each value as its distance from the mean, measured in sample standard
/// deviations. Flat data has no spread to divide by.
pub fn zscores(values: &Vec<f64>) -> Result<Vec<f64>, String> {
    if values.len() < 2 {
        return Err(format!("stats_zscores: needs at least two values to measure spread, got {}", values.len()));
    }
    let average = mean(values)?;
    let spread = stddev(values)?;
    if spread == 0.0 {
        return Err("stats_zscores: the array holds the same value throughout, so every z-score would divide by zero".to_string());
    }
    return Ok(values.iter().map(|value| (value - average) / spread).collect());
}

/// Each value scaled linearly onto 0.0..1.0, the smallest becoming 0.0 and the
/// largest 1.0. Flat data has no width to scale by.
pub fn normalize(values: &Vec<f64>) -> Result<Vec<f64>, String> {
    require_values("stats_normalize", values)?;
    let (low, high) = bounds(values);
    if low == high {
        return Err("stats_normalize: the array holds the same value throughout, so it cannot be scaled to 0.0..1.0".to_string());
    }
    return Ok(values.iter().map(|value| (value - low) / (high - low)).collect());
}

/// The running total after each value. An empty array stays empty.
pub fn cumulative_sum(values: &Vec<f64>) -> Vec<f64> {
    let mut running = 0.0;
    return values
        .iter()
        .map(|value| {
            running += value;
            running
        })
        .collect();
}

/// The step from each value to the next, one shorter than the input - the
/// discrete derivative of a series, and the inverse of the cumulative sum.
pub fn differences(values: &Vec<f64>) -> Vec<f64> {
    return values.windows(2).map(|pair| pair[1] - pair[0]).collect();
}

/// The percent change from each value to the next, one shorter than the
/// input. A step cannot start from zero, since any change from zero would be
/// infinite.
pub fn percent_change(values: &Vec<f64>) -> Result<Vec<f64>, String> {
    let mut changes = Vec::new();
    for pair in values.windows(2) {
        if pair[0] == 0.0 {
            return Err("stats_percent_change: a value of zero has no percent change to the next value".to_string());
        }
        changes.push((pair[1] - pair[0]) / pair[0] * 100.0);
    }
    return Ok(changes);
}

/// The mean of each window-sized run of neighbouring values, smoothing a
/// noisy series. The window must fit inside the array, and the output has
/// len - window + 1 values.
pub fn moving_average(values: &Vec<f64>, window: i64) -> Result<Vec<f64>, String> {
    require_values("stats_moving_average", values)?;
    if window < 1 || window as usize > values.len() {
        return Err(format!("stats_moving_average: a window of {} does not fit an array of {} values", window, values.len()));
    }
    let size = window as usize;
    return Ok(values.windows(size).map(|run| run.iter().sum::<f64>() / size as f64).collect());
}

/// Exponentially weighted moving average: each output blends the newest value
/// with everything before it. A smoothing factor of 1.0 tracks the data
/// exactly; factors near 0.0 smooth heavily and react slowly.
pub fn ewma(values: &Vec<f64>, alpha: f64) -> Result<Vec<f64>, String> {
    require_values("stats_ewma", values)?;
    if !(alpha > 0.0 && alpha <= 1.0) {
        return Err(format!("stats_ewma: {} is not a smoothing factor above 0.0 and at most 1.0", alpha));
    }
    let mut smoothed = Vec::with_capacity(values.len());
    let mut current = values[0];
    smoothed.push(current);
    for value in values.iter().skip(1) {
        current = alpha * value + (1.0 - alpha) * current;
        smoothed.push(current);
    }
    return Ok(smoothed);
}

/// Counts per equal-width bin spanning the smallest value to the largest, the
/// largest value landing in the last bin rather than falling off the end.
/// Flat data has no width to divide, so it all lands in bin zero.
pub fn histogram(values: &Vec<f64>, bins: i64) -> Result<Vec<i64>, String> {
    require_values("stats_histogram", values)?;
    if bins < 1 {
        return Err(format!("stats_histogram: {} bins makes no sense, at least one is needed", bins));
    }
    let (low, high) = bounds(values);
    let mut counts = vec![0i64; bins as usize];
    if low == high {
        counts[0] = values.len() as i64;
        return Ok(counts);
    }
    let width = (high - low) / bins as f64;
    for value in values.iter() {
        let mut index = ((value - low) / width).floor() as i64;
        if index >= bins {
            index = bins - 1;
        }
        counts[index as usize] += 1;
    }
    return Ok(counts);
}

/// The values beyond 1.5 interquartile ranges outside the quartiles - the
/// boxplot rule - reported in their original order. Fences drawn from fewer
/// than four values say more about the sample size than the data, so a short
/// array reports no outliers, and an empty one stays empty.
pub fn outliers(values: &Vec<f64>) -> Vec<f64> {
    if values.len() < 4 {
        return Vec::new();
    }
    let (lower_quartile, upper_quartile) = match (percentile(values, 0.25), percentile(values, 0.75)) {
        (Ok(lower), Ok(upper)) => (lower, upper),
        // Unreachable: the array is non-empty and both shares are in range.
        _ => return Vec::new(),
    };
    let margin = 1.5 * (upper_quartile - lower_quartile);
    return values.iter().filter(|value| **value < lower_quartile - margin || **value > upper_quartile + margin).cloned().collect();
}

/// The share of values at or below the target, from 0.0 to 100.0 - the
/// inverse question to stats_percentile.
pub fn percentile_rank(values: &Vec<f64>, target: f64) -> Result<f64, String> {
    require_values("stats_percentile_rank", values)?;
    let at_or_below = values.iter().filter(|value| **value <= target).count();
    return Ok(at_or_below as f64 / values.len() as f64 * 100.0);
}

/// The 25th, 50th and 75th percentiles as a three-value array - the box of a
/// boxplot in one call.
pub fn quartiles(values: &Vec<f64>) -> Result<Vec<f64>, String> {
    require_values("stats_quartiles", values)?;
    return Ok(vec![percentile(values, 0.25)?, percentile(values, 0.5)?, percentile(values, 0.75)?]);
}

/// The standard normal cdf, the probability of a draw at or below z.
/// Routed through the complementary error function so both tails keep
/// their relative accuracy instead of rounding to 0 or 1 early.
fn standard_normal_cdf(z: f64) -> f64 {
    return 0.5 * erfc(-z / std::f64::consts::SQRT_2);
}

/// The standard normal quantile: Acklam's rational starting guess, good to
/// about nine digits on its own, then one Halley step against the cdf
/// above so the pair round trips to machine precision. The caller
/// guarantees the probability is strictly between 0 and 1.
fn standard_normal_inverse(probability: f64) -> f64 {
    const CENTER_NUM: [f64; 6] = [-3.969683028665376e+01, 2.209460984245205e+02, -2.759285104469687e+02, 1.383577518672690e+02, -3.066479806614716e+01, 2.506628277459239e+00];
    const CENTER_DEN: [f64; 5] = [-5.447609879822406e+01, 1.615858368580409e+02, -1.556989798598866e+02, 6.680131188771972e+01, -1.328068155288572e+01];
    const TAIL_NUM: [f64; 6] = [-7.784894002430293e-03, -3.223964580411365e-01, -2.400758277161838e+00, -2.549732539343734e+00, 4.374664141464968e+00, 2.938163982698783e+00];
    const TAIL_DEN: [f64; 4] = [7.784695709041462e-03, 3.224671290700398e-01, 2.445134137142996e+00, 3.754408661907416e+00];
    const LOWER_EDGE: f64 = 0.02425;

    let tail_estimate = |q: f64| {
        return (((((TAIL_NUM[0] * q + TAIL_NUM[1]) * q + TAIL_NUM[2]) * q + TAIL_NUM[3]) * q + TAIL_NUM[4]) * q + TAIL_NUM[5])
            / ((((TAIL_DEN[0] * q + TAIL_DEN[1]) * q + TAIL_DEN[2]) * q + TAIL_DEN[3]) * q + 1.0);
    };

    let mut z: f64;
    if probability < LOWER_EDGE {
        z = tail_estimate((-2.0 * probability.ln()).sqrt());
    } else if probability <= 1.0 - LOWER_EDGE {
        let q = probability - 0.5;
        let r = q * q;
        z = (((((CENTER_NUM[0] * r + CENTER_NUM[1]) * r + CENTER_NUM[2]) * r + CENTER_NUM[3]) * r + CENTER_NUM[4]) * r + CENTER_NUM[5]) * q
            / (((((CENTER_DEN[0] * r + CENTER_DEN[1]) * r + CENTER_DEN[2]) * r + CENTER_DEN[3]) * r + CENTER_DEN[4]) * r + 1.0);
    } else {
        // ln_1p keeps 1 - p accurate right up against 1.
        z = -tail_estimate((-2.0 * (-probability).ln_1p()).sqrt());
    }

    let density = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    if density > 0.0 {
        let miss = standard_normal_cdf(z) - probability;
        let step = miss / density;
        let refined = z - step / (1.0 + z * step / 2.0);
        if refined.is_finite() {
            z = refined;
        }
    }
    return z;
}

/// Shared guard for the normal distribution functions, so a flat or upside
/// down bell fails the same way everywhere.
fn require_positive_stddev(function: &str, stddev: f64) -> Result<(), String> {
    if !(stddev > 0.0) {
        return Err(format!("{}: the standard deviation is {} and must be positive", function, stddev));
    }
    return Ok(());
}

/// The probability that a draw from the given normal distribution lands at
/// or below the value.
pub fn normal_cdf(value: f64, mean: f64, stddev: f64) -> Result<f64, String> {
    require_positive_stddev("stats_normal_cdf", stddev)?;
    return Ok(standard_normal_cdf((value - mean) / stddev));
}

/// The value below which the given share of a normal distribution falls,
/// the inverse of normal_cdf. The probability must be strictly between 0
/// and 1, since no finite value has all of the distribution on one side.
pub fn normal_inverse(probability: f64, mean: f64, stddev: f64) -> Result<f64, String> {
    require_positive_stddev("stats_normal_inverse", stddev)?;
    if !(probability > 0.0 && probability < 1.0) {
        return Err(format!("stats_normal_inverse: {} is not a probability strictly between 0 and 1", probability));
    }
    return Ok(mean + stddev * standard_normal_inverse(probability));
}

/// The height of the normal bell curve at the value. A density rather than
/// a probability, so it can top 1.0 when the curve is narrow.
pub fn normal_pdf(value: f64, mean: f64, stddev: f64) -> Result<f64, String> {
    require_positive_stddev("stats_normal_pdf", stddev)?;
    let z = (value - mean) / stddev;
    return Ok((-0.5 * z * z).exp() / (stddev * (2.0 * std::f64::consts::PI).sqrt()));
}

/// The natural log of the gamma function for positive arguments, via the
/// Lanczos approximation, good to about ten digits. Only used for counts
/// too large to sum logarithms over one at a time.
fn ln_gamma(x: f64) -> f64 {
    const COEFFICIENTS: [f64; 6] = [76.18009172947146, -86.50532032941677, 24.01409824083091, -1.231739572450155, 0.1208650973866179e-2, -0.5395239384953e-5];
    let mut series = 1.000000000190015;
    for (index, coefficient) in COEFFICIENTS.iter().enumerate() {
        series += coefficient / (x + 1.0 + index as f64);
    }
    let shifted = x + 5.5;
    return -shifted + (x + 0.5) * shifted.ln() + (2.5066282746310005 * series / x).ln();
}

/// The natural log of n choose k, as a running sum of log ratios for
/// everyday sizes and from the gamma function for counts too large to loop
/// over. The caller guarantees 0 <= k <= n.
fn ln_choose(n: i64, k: i64) -> f64 {
    let smaller = k.min(n - k);
    if smaller > 1_000_000 {
        return ln_gamma(n as f64 + 1.0) - ln_gamma(k as f64 + 1.0) - ln_gamma((n - k) as f64 + 1.0);
    }
    let mut total = 0.0;
    for step in 1..=smaller {
        total += ((n - smaller + step) as f64 / step as f64).ln();
    }
    return total;
}

/// The natural log of a factorial, summed directly for everyday counts and
/// from the gamma function beyond them. The caller guarantees the count is
/// not negative.
fn ln_factorial(count: i64) -> f64 {
    if count > 1_000_000 {
        return ln_gamma(count as f64 + 1.0);
    }
    let mut total = 0.0;
    for step in 2..=count {
        total += (step as f64).ln();
    }
    return total;
}

/// Shared guard for the binomial functions, so both check their bounds the
/// same way.
fn require_binomial(function: &str, successes: i64, trials: i64, probability: f64) -> Result<(), String> {
    if trials < 0 {
        return Err(format!("{}: the trial count is {} and cannot be negative", function, trials));
    }
    if successes < 0 || successes > trials {
        return Err(format!("{}: {} successes cannot come out of {} trials, the count runs from 0 to the number of trials", function, successes, trials));
    }
    if !(0.0..=1.0).contains(&probability) {
        return Err(format!("{}: {} is not a probability between 0.0 and 1.0", function, probability));
    }
    return Ok(());
}

/// The probability of exactly that many successes in that many independent
/// tries, each succeeding with the given probability. Assembled in log
/// space, so a thousand-trial case whose factorials would overflow any
/// float comes out exact.
pub fn binomial_pmf(successes: i64, trials: i64, probability: f64) -> Result<f64, String> {
    require_binomial("stats_binomial_pmf", successes, trials, probability)?;
    if probability == 0.0 {
        return Ok(if successes == 0 { 1.0 } else { 0.0 });
    }
    if probability == 1.0 {
        return Ok(if successes == trials { 1.0 } else { 0.0 });
    }
    let log_mass = ln_choose(trials, successes) + successes as f64 * probability.ln() + (trials - successes) as f64 * (-probability).ln_1p();
    return Ok(log_mass.exp());
}

/// The probability of at most that many successes, the binomial pmf summed
/// from zero upward. Each term follows from the last by one multiply in
/// log space, so long runs stay cheap and never overflow.
pub fn binomial_cdf(successes: i64, trials: i64, probability: f64) -> Result<f64, String> {
    require_binomial("stats_binomial_cdf", successes, trials, probability)?;
    if probability == 0.0 {
        return Ok(1.0);
    }
    if probability == 1.0 {
        return Ok(if successes == trials { 1.0 } else { 0.0 });
    }
    let log_odds = probability.ln() - (-probability).ln_1p();
    let mut log_term = trials as f64 * (-probability).ln_1p();
    let mut total = log_term.exp();
    for hit in 1..=successes {
        log_term += ((trials - hit + 1) as f64).ln() - (hit as f64).ln() + log_odds;
        total += log_term.exp();
    }
    return Ok(total.min(1.0));
}

/// Shared guard for the Poisson functions, so both check their bounds the
/// same way.
fn require_poisson(function: &str, events: i64, rate: f64) -> Result<(), String> {
    if !(rate > 0.0) {
        return Err(format!("{}: the rate is {} and must be positive", function, rate));
    }
    if events < 0 {
        return Err(format!("{}: the event count is {} and cannot be negative", function, events));
    }
    return Ok(());
}

/// The probability of exactly that many events when they arrive
/// independently at the given average rate. Assembled in log space, so a
/// large count cannot overflow the factorial on the way to a small answer.
pub fn poisson_pmf(events: i64, rate: f64) -> Result<f64, String> {
    require_poisson("stats_poisson_pmf", events, rate)?;
    return Ok((events as f64 * rate.ln() - rate - ln_factorial(events)).exp());
}

/// The probability of at most that many events, the Poisson pmf summed
/// from zero upward in log space.
pub fn poisson_cdf(events: i64, rate: f64) -> Result<f64, String> {
    require_poisson("stats_poisson_cdf", events, rate)?;
    let mut log_term = -rate;
    let mut total = log_term.exp();
    for count in 1..=events {
        log_term += rate.ln() - (count as f64).ln();
        total += log_term.exp();
    }
    return Ok(total.min(1.0));
}

/// How many people a survey needs so an estimated proportion lands within
/// the margin of error at the given confidence, assuming the worst case
/// proportion of one half and rounding up. The classic poll planning
/// number: a 3 percent margin at 95 percent confidence asks for 1068.
pub fn sample_size_for_proportion(margin_of_error: f64, confidence: f64) -> Result<i64, String> {
    if !(margin_of_error > 0.0 && margin_of_error < 1.0) {
        return Err(format!("stats_sample_size_for_proportion: {} is not a margin of error strictly between 0 and 1", margin_of_error));
    }
    if !(confidence > 0.0 && confidence < 1.0) {
        return Err(format!("stats_sample_size_for_proportion: {} is not a confidence level strictly between 0 and 1", confidence));
    }
    // The two-sided critical value, asked of the lower tail because
    // (1 - confidence) / 2 stays exact where (1 + confidence) / 2 rounds.
    let critical = -standard_normal_inverse((1.0 - confidence) / 2.0);
    let required = (critical * critical * 0.25 / (margin_of_error * margin_of_error)).ceil();
    if !(required <= i64::MAX as f64) {
        return Err("stats_sample_size_for_proportion: the required sample size overflows a 64-bit integer".to_string());
    }
    return Ok(required as i64);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 1e-9;
    }

    #[test]
    fn mean_averages() {
        assert!(close(mean(&vec![1.0, 2.0, 3.0, 4.0]).expect("values"), 2.5));
    }

    #[test]
    fn median_handles_both_odd_and_even_counts() {
        assert!(close(median(&vec![3.0, 1.0, 2.0]).expect("values"), 2.0));
        assert!(close(median(&vec![4.0, 1.0, 3.0, 2.0]).expect("values"), 2.5));
    }

    #[test]
    fn median_ignores_an_outlier_the_mean_chases() {
        let values = vec![1.0, 2.0, 3.0, 1000.0];
        assert!(close(median(&values).expect("values"), 2.5));
        assert!(mean(&values).expect("values") > 200.0);
    }

    #[test]
    fn variance_and_stddev_use_the_sample_divisor() {
        // Sample variance of 2,4,4,4,5,5,7,9 is 4.571428..., not the
        // population variance of 4.
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!(close(variance(&values).expect("values"), 32.0 / 7.0));
        assert!(close(stddev(&values).expect("values"), (32.0f64 / 7.0).sqrt()));
    }

    #[test]
    fn spread_needs_two_values() {
        assert!(variance(&vec![1.0]).unwrap_err().contains("at least two values"));
        assert!(stddev(&vec![1.0]).unwrap_err().contains("at least two values"));
    }

    #[test]
    fn percentile_interpolates_between_neighbours() {
        let values = vec![1.0, 2.0, 3.0, 4.0];
        assert!(close(percentile(&values, 0.0).expect("values"), 1.0));
        assert!(close(percentile(&values, 1.0).expect("values"), 4.0));
        assert!(close(percentile(&values, 0.5).expect("values"), 2.5));
    }

    #[test]
    fn percentile_rejects_a_share_outside_zero_to_one() {
        assert!(percentile(&vec![1.0], 95.0).unwrap_err().contains("not a share"));
    }

    #[test]
    fn mode_is_the_most_common_value() {
        assert!(close(mode(&vec![1.0, 2.0, 2.0, 3.0]).expect("values"), 2.0));
        assert!(close(mode(&vec![5.0]).expect("values"), 5.0));
    }

    #[test]
    fn mode_breaks_a_tie_with_the_smallest_value() {
        assert!(close(mode(&vec![9.0, 9.0, 1.0, 1.0, 4.0]).expect("values"), 1.0));
        // Every value appearing once is a tie between all of them.
        assert!(close(mode(&vec![3.0, 1.0, 2.0]).expect("values"), 1.0));
    }

    #[test]
    fn range_is_the_distance_between_the_extremes() {
        assert!(close(range(&vec![4.0, -2.0, 9.0]).expect("values"), 11.0));
    }

    #[test]
    fn correlation_is_one_for_a_straight_line() {
        let first = vec![1.0, 2.0, 3.0, 4.0];
        let second = vec![2.0, 4.0, 6.0, 8.0];
        assert!(close(correlation(&first, &second).expect("pairs"), 1.0));
    }

    #[test]
    fn correlation_is_minus_one_when_one_falls_as_the_other_rises() {
        let first = vec![1.0, 2.0, 3.0, 4.0];
        let second = vec![8.0, 6.0, 4.0, 2.0];
        assert!(close(correlation(&first, &second).expect("pairs"), -1.0));
    }

    #[test]
    fn correlation_rejects_mismatched_lengths_and_flat_columns() {
        assert!(correlation(&vec![1.0, 2.0], &vec![1.0]).unwrap_err().contains("same length"));
        assert!(correlation(&vec![1.0, 1.0], &vec![1.0, 2.0]).unwrap_err().contains("same value throughout"));
    }

    #[test]
    fn every_summary_rejects_an_empty_array() {
        let empty: Vec<f64> = vec![];
        assert!(mean(&empty).unwrap_err().contains("empty"));
        assert!(median(&empty).unwrap_err().contains("empty"));
        assert!(range(&empty).unwrap_err().contains("empty"));
        assert!(percentile(&empty, 0.5).unwrap_err().contains("empty"));
    }

    fn all_close(left: &Vec<f64>, right: &Vec<f64>) -> bool {
        return left.len() == right.len() && left.iter().zip(right.iter()).all(|(a, b)| close(*a, *b));
    }

    #[test]
    fn geometric_mean_is_the_root_of_the_product() {
        assert!(close(geometric_mean(&vec![2.0, 8.0]).expect("values"), 4.0));
        assert!(close(geometric_mean(&vec![1.0, 3.0, 9.0]).expect("values"), 3.0));
    }

    #[test]
    fn harmonic_mean_averages_rates() {
        // Python's statistics docs example: two legs at 40 and 60 average 48.
        assert!(close(harmonic_mean(&vec![40.0, 60.0]).expect("values"), 48.0));
        assert!(close(harmonic_mean(&vec![1.0, 4.0, 4.0]).expect("values"), 2.0));
    }

    #[test]
    fn geometric_and_harmonic_means_demand_positive_values() {
        assert!(geometric_mean(&vec![1.0, -2.0]).unwrap_err().contains("not positive"));
        assert!(geometric_mean(&vec![0.0]).unwrap_err().contains("not positive"));
        assert!(harmonic_mean(&vec![1.0, 0.0]).unwrap_err().contains("not positive"));
    }

    #[test]
    fn weighted_mean_counts_values_by_their_weights() {
        // (1*1 + 2*1 + 3*2) / 4 = 2.25.
        assert!(close(weighted_mean(&vec![1.0, 2.0, 3.0], &vec![1.0, 1.0, 2.0]).expect("values"), 2.25));
    }

    #[test]
    fn weighted_mean_rejects_mismatch_and_weightless_weights() {
        assert!(weighted_mean(&vec![1.0, 2.0], &vec![1.0]).unwrap_err().contains("every value needs one"));
        assert!(weighted_mean(&vec![1.0, 2.0], &vec![0.0, 0.0]).unwrap_err().contains("positive total"));
        assert!(weighted_mean(&vec![1.0, 2.0], &vec![-1.0, -1.0]).unwrap_err().contains("positive total"));
    }

    #[test]
    fn trimmed_mean_drops_both_ends() {
        // Sorted [-50,1,2,3,4,10] with floor(6*0.2)=1 dropped per end keeps [1,2,3,4].
        let values = vec![10.0, 1.0, 2.0, 3.0, 4.0, -50.0];
        assert!(close(trimmed_mean(&values, 0.2).expect("values"), 2.5));
        // A zero share trims nothing, so it is the plain mean.
        assert!(close(trimmed_mean(&values, 0.0).expect("values"), mean(&values).expect("values")));
    }

    #[test]
    fn trimmed_mean_rejects_a_share_at_or_beyond_half() {
        assert!(trimmed_mean(&vec![1.0, 2.0], 0.5).unwrap_err().contains("not a trim share"));
        assert!(trimmed_mean(&vec![1.0, 2.0], -0.1).unwrap_err().contains("not a trim share"));
    }

    #[test]
    fn iqr_is_the_width_of_the_middle_half() {
        // Quartiles of [1,2,3,4] interpolate to 1.75 and 3.25.
        assert!(close(iqr(&vec![1.0, 2.0, 3.0, 4.0]).expect("values"), 1.5));
    }

    #[test]
    fn mad_is_the_median_distance_from_the_median() {
        // Median 3, distances [2,1,0,1,2], median distance 1.
        assert!(close(mad(&vec![1.0, 2.0, 3.0, 4.0, 5.0]).expect("values"), 1.0));
    }

    #[test]
    fn skewness_is_zero_for_symmetric_data_and_positive_for_a_right_tail() {
        assert!(close(skewness(&vec![1.0, 2.0, 3.0, 4.0, 5.0]).expect("values"), 0.0));
        // Hand-checked against the adjusted formula: works out to exactly 1.2*sqrt(2).
        assert!(close(skewness(&vec![1.0, 2.0, 3.0, 4.0, 10.0]).expect("values"), 1.2 * 2.0_f64.sqrt()));
    }

    #[test]
    fn kurtosis_is_negative_for_uniform_data_and_positive_for_a_peak() {
        // Excel KURT(1,2,3,4,5) = -1.2.
        assert!(close(kurtosis(&vec![1.0, 2.0, 3.0, 4.0, 5.0]).expect("values"), -1.2));
        // A tight peak with two stragglers: works out to exactly 3.
        assert!(close(kurtosis(&vec![1.0, 2.0, 2.0, 2.0, 2.0, 2.0, 3.0]).expect("values"), 3.0));
    }

    #[test]
    fn skewness_and_kurtosis_need_enough_varied_values() {
        assert!(skewness(&vec![1.0, 2.0]).unwrap_err().contains("at least three"));
        assert!(kurtosis(&vec![1.0, 2.0, 3.0]).unwrap_err().contains("at least four"));
        assert!(skewness(&vec![5.0, 5.0, 5.0]).unwrap_err().contains("same value throughout"));
        assert!(kurtosis(&vec![5.0, 5.0, 5.0, 5.0]).unwrap_err().contains("same value throughout"));
    }

    #[test]
    fn sem_shrinks_the_stddev_by_the_root_of_the_count() {
        // Stddev of [1,2,3] is 1, so the standard error is 1/sqrt(3).
        assert!(close(sem(&vec![1.0, 2.0, 3.0]).expect("values"), 1.0 / 3.0_f64.sqrt()));
        assert!(sem(&vec![1.0]).unwrap_err().contains("at least two values"));
    }

    #[test]
    fn cv_is_spread_as_a_share_of_the_mean() {
        // Stddev 1 over mean 2.
        assert!(close(cv(&vec![1.0, 2.0, 3.0]).expect("values"), 0.5));
        assert!(cv(&vec![1.0]).unwrap_err().contains("at least two values"));
    }

    #[test]
    fn cv_rejects_a_zero_mean() {
        assert!(cv(&vec![-1.0, 0.0, 1.0]).unwrap_err().contains("mean is zero"));
    }

    #[test]
    fn population_spread_divides_by_n() {
        // Mean 2.5, squared deviations sum to 5, over n=4.
        assert!(close(pvariance(&vec![1.0, 2.0, 3.0, 4.0]).expect("values"), 1.25));
        assert!(close(pstddev(&vec![1.0, 2.0, 3.0, 4.0]).expect("values"), 1.25_f64.sqrt()));
        // A single value is its own population, with no spread.
        assert!(close(pvariance(&vec![5.0]).expect("values"), 0.0));
    }

    #[test]
    fn rms_measures_magnitude_ignoring_sign() {
        assert!(close(rms(&vec![3.0, 4.0]).expect("values"), 12.5_f64.sqrt()));
        // The plain mean of [-3,3] is 0; the rms is 3.
        assert!(close(rms(&vec![-3.0, 3.0]).expect("values"), 3.0));
    }

    #[test]
    fn midrange_is_the_midpoint_of_the_extremes() {
        assert!(close(midrange(&vec![1.0, 9.0, 3.0]).expect("values"), 5.0));
    }

    #[test]
    fn covariance_of_a_doubling_column_is_two() {
        // Textbook pair: deltas (-1,0,1) and (-2,0,2) give 4/(n-1) = 2.
        assert!(close(covariance(&vec![1.0, 2.0, 3.0], &vec![2.0, 4.0, 6.0]).expect("pairs"), 2.0));
    }

    #[test]
    fn covariance_rejects_mismatched_lengths_and_single_pairs() {
        assert!(covariance(&vec![1.0, 2.0], &vec![1.0]).unwrap_err().contains("same length"));
        assert!(covariance(&vec![1.0], &vec![1.0]).unwrap_err().contains("at least two pairs"));
    }

    #[test]
    fn rank_averages_ties_and_keeps_the_original_order() {
        assert!(all_close(&rank(&vec![3.0, 1.0, 2.0]), &vec![3.0, 1.0, 2.0]));
        assert!(all_close(&rank(&vec![10.0, 20.0, 20.0, 30.0]), &vec![1.0, 2.5, 2.5, 4.0]));
        assert!(rank(&vec![]).is_empty());
    }

    #[test]
    fn spearman_sees_a_monotone_curve_as_a_perfect_link() {
        let inputs = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let squares = vec![1.0, 4.0, 9.0, 16.0, 25.0];
        assert!(close(spearman(&inputs, &squares).expect("pairs"), 1.0));
        assert!(close(spearman(&vec![1.0, 2.0, 3.0], &vec![9.0, 4.0, 1.0]).expect("pairs"), -1.0));
    }

    #[test]
    fn spearman_rejects_mismatched_lengths_and_flat_columns() {
        assert!(spearman(&vec![1.0, 2.0], &vec![1.0]).unwrap_err().contains("same length"));
        assert!(spearman(&vec![1.0], &vec![1.0]).unwrap_err().contains("at least two pairs"));
        let flat_error = spearman(&vec![1.0, 1.0, 1.0], &vec![1.0, 2.0, 3.0]).unwrap_err();
        assert!(flat_error.starts_with("stats_spearman"));
        assert!(flat_error.contains("same value throughout"));
    }

    #[test]
    fn zscores_center_and_scale_the_data() {
        assert!(all_close(&zscores(&vec![1.0, 2.0, 3.0]).expect("values"), &vec![-1.0, 0.0, 1.0]));
    }

    #[test]
    fn zscores_and_normalize_reject_flat_data() {
        assert!(zscores(&vec![5.0, 5.0]).unwrap_err().contains("same value throughout"));
        assert!(zscores(&vec![5.0]).unwrap_err().contains("at least two values"));
        assert!(normalize(&vec![5.0, 5.0]).unwrap_err().contains("same value throughout"));
    }

    #[test]
    fn normalize_scales_onto_zero_to_one() {
        assert!(all_close(&normalize(&vec![2.0, 4.0, 6.0]).expect("values"), &vec![0.0, 0.5, 1.0]));
    }

    #[test]
    fn cumulative_sum_keeps_a_running_total() {
        assert!(all_close(&cumulative_sum(&vec![1.0, 2.0, 3.0]), &vec![1.0, 3.0, 6.0]));
        assert!(cumulative_sum(&vec![]).is_empty());
    }

    #[test]
    fn differences_step_between_neighbours() {
        assert!(all_close(&differences(&vec![5.0, 3.0, 8.0]), &vec![-2.0, 5.0]));
        assert!(differences(&vec![7.0]).is_empty());
        assert!(differences(&vec![]).is_empty());
    }

    #[test]
    fn percent_change_reports_each_step_as_a_percentage() {
        assert!(all_close(&percent_change(&vec![100.0, 110.0, 99.0]).expect("values"), &vec![10.0, -10.0]));
        assert!(percent_change(&vec![50.0]).expect("values").is_empty());
    }

    #[test]
    fn percent_change_rejects_a_zero_base() {
        assert!(percent_change(&vec![0.0, 5.0]).unwrap_err().contains("zero"));
    }

    #[test]
    fn moving_average_smooths_with_a_sliding_window() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(all_close(&moving_average(&values, 2).expect("values"), &vec![1.5, 2.5, 3.5, 4.5]));
        // A window the size of the array is just the mean.
        assert!(all_close(&moving_average(&values, 5).expect("values"), &vec![3.0]));
    }

    #[test]
    fn moving_average_window_must_fit() {
        assert!(moving_average(&vec![1.0, 2.0], 0).unwrap_err().contains("does not fit"));
        assert!(moving_average(&vec![1.0, 2.0], 3).unwrap_err().contains("does not fit"));
    }

    #[test]
    fn ewma_blends_new_values_into_the_running_average() {
        assert!(all_close(&ewma(&vec![1.0, 2.0, 3.0], 0.5).expect("values"), &vec![1.0, 1.5, 2.25]));
        // A factor of 1.0 tracks the data exactly.
        assert!(all_close(&ewma(&vec![4.0, 7.0, 2.0], 1.0).expect("values"), &vec![4.0, 7.0, 2.0]));
    }

    #[test]
    fn ewma_rejects_a_smoothing_factor_outside_range() {
        assert!(ewma(&vec![1.0], 0.0).unwrap_err().contains("smoothing factor"));
        assert!(ewma(&vec![1.0], 1.5).unwrap_err().contains("smoothing factor"));
    }

    #[test]
    fn histogram_counts_into_equal_width_bins() {
        // Width (4-1)/2 = 1.5: values 1,2 land low, 3 lands high, and the top
        // value 4 lands in the last bin rather than off the end.
        assert_eq!(histogram(&vec![1.0, 2.0, 3.0, 4.0], 2).expect("values"), vec![2, 2]);
    }

    #[test]
    fn histogram_puts_flat_data_in_bin_zero() {
        assert_eq!(histogram(&vec![5.0, 5.0, 5.0], 3).expect("values"), vec![3, 0, 0]);
    }

    #[test]
    fn histogram_needs_at_least_one_bin() {
        assert!(histogram(&vec![1.0], 0).unwrap_err().contains("at least one"));
    }

    #[test]
    fn outliers_apply_the_boxplot_fences_in_original_order() {
        // Quartiles 2 and 4, fences -1 and 7.
        assert!(all_close(&outliers(&vec![1.0, 2.0, 3.0, 4.0, 100.0]), &vec![100.0]));
        assert!(all_close(&outliers(&vec![100.0, 1.0, 2.0, 3.0, 4.0]), &vec![100.0]));
        assert!(all_close(&outliers(&vec![-100.0, 1.0, 2.0, 3.0, 4.0]), &vec![-100.0]));
    }

    #[test]
    fn outliers_report_none_for_short_or_empty_arrays() {
        assert!(outliers(&vec![1.0, 2.0, 1000.0]).is_empty());
        assert!(outliers(&vec![]).is_empty());
    }

    #[test]
    fn percentile_rank_is_the_share_at_or_below_the_target() {
        assert!(close(percentile_rank(&vec![1.0, 2.0, 3.0, 4.0], 3.0).expect("values"), 75.0));
        assert!(close(percentile_rank(&vec![1.0, 2.0, 3.0], 1.5).expect("values"), 100.0 / 3.0));
        assert!(close(percentile_rank(&vec![1.0, 2.0, 3.0], 0.0).expect("values"), 0.0));
        assert!(close(percentile_rank(&vec![1.0, 2.0, 3.0], 9.0).expect("values"), 100.0));
    }

    #[test]
    fn quartiles_return_the_box_of_a_boxplot() {
        assert!(all_close(&quartiles(&vec![1.0, 2.0, 3.0, 4.0]).expect("values"), &vec![1.75, 2.5, 3.25]));
    }

    #[test]
    fn every_new_summary_rejects_an_empty_array() {
        let empty: Vec<f64> = vec![];
        assert!(geometric_mean(&empty).unwrap_err().contains("empty"));
        assert!(harmonic_mean(&empty).unwrap_err().contains("empty"));
        assert!(weighted_mean(&empty, &empty).unwrap_err().contains("empty"));
        assert!(trimmed_mean(&empty, 0.1).unwrap_err().contains("empty"));
        assert!(iqr(&empty).unwrap_err().contains("empty"));
        assert!(mad(&empty).unwrap_err().contains("empty"));
        assert!(pvariance(&empty).unwrap_err().contains("empty"));
        assert!(pstddev(&empty).unwrap_err().contains("empty"));
        assert!(rms(&empty).unwrap_err().contains("empty"));
        assert!(midrange(&empty).unwrap_err().contains("empty"));
        assert!(normalize(&empty).unwrap_err().contains("empty"));
        assert!(moving_average(&empty, 1).unwrap_err().contains("empty"));
        assert!(ewma(&empty, 0.5).unwrap_err().contains("empty"));
        assert!(histogram(&empty, 2).unwrap_err().contains("empty"));
        assert!(percentile_rank(&empty, 1.0).unwrap_err().contains("empty"));
        assert!(quartiles(&empty).unwrap_err().contains("empty"));
    }

    #[test]
    fn normal_cdf_matches_the_z_table() {
        assert_eq!(normal_cdf(0.0, 0.0, 1.0).expect("a positive stddev"), 0.5, "the mean sits exactly at the median");
        assert!((normal_cdf(1.96, 0.0, 1.0).expect("a positive stddev") - 0.9750021048517795).abs() < 1e-9);
        assert!((normal_cdf(-1.96, 0.0, 1.0).expect("a positive stddev") - 0.0249978951482205).abs() < 1e-9);
        assert!((normal_cdf(1.0, 0.0, 1.0).expect("a positive stddev") - 0.8413447460685429).abs() < 1e-9);
        // Scaling and shifting: 115 under N(100, 15) is one stddev up.
        assert!(close(normal_cdf(115.0, 100.0, 15.0).expect("a positive stddev"), normal_cdf(1.0, 0.0, 1.0).expect("a positive stddev")));
    }

    #[test]
    fn normal_pdf_peaks_at_the_mean_and_scales_with_the_spread() {
        assert!((normal_pdf(0.0, 0.0, 1.0).expect("a positive stddev") - 0.3989422804014327).abs() < 1e-12);
        assert!((normal_pdf(1.0, 1.0, 2.0).expect("a positive stddev") - 0.19947114020071635).abs() < 1e-12);
        assert!(normal_pdf(0.0, 0.0, 1.0).expect("a positive stddev") > normal_pdf(1.0, 0.0, 1.0).expect("a positive stddev"));
    }

    #[test]
    fn normal_inverse_hits_the_critical_values_and_round_trips_the_cdf() {
        assert!((normal_inverse(0.975, 0.0, 1.0).expect("in range") - 1.9599639845400545).abs() < 1e-8);
        assert_eq!(normal_inverse(0.5, 0.0, 1.0).expect("in range"), 0.0);
        assert!((normal_inverse(0.975, 100.0, 15.0).expect("in range") - 129.39945976810082).abs() < 1e-6);
        for probability in [0.001, 0.025, 0.5, 0.975, 0.999] {
            let cutoff = normal_inverse(probability, 0.0, 1.0).expect("in range");
            let round_trip = normal_cdf(cutoff, 0.0, 1.0).expect("a positive stddev");
            assert!((round_trip - probability).abs() < 1e-7, "failed at {}", probability);
        }
    }

    #[test]
    fn the_normal_functions_reject_a_flat_or_negative_spread() {
        assert!(normal_cdf(1.0, 0.0, 0.0).unwrap_err().contains("must be positive"));
        assert!(normal_pdf(1.0, 0.0, -1.0).unwrap_err().contains("must be positive"));
        assert!(normal_inverse(0.5, 0.0, 0.0).unwrap_err().contains("must be positive"));
    }

    #[test]
    fn normal_inverse_rejects_a_probability_at_or_beyond_the_ends() {
        assert!(normal_inverse(0.0, 0.0, 1.0).unwrap_err().contains("strictly between"));
        assert!(normal_inverse(1.0, 0.0, 1.0).unwrap_err().contains("strictly between"));
        assert!(normal_inverse(-0.5, 0.0, 1.0).unwrap_err().contains("strictly between"));
        assert!(normal_inverse(1.5, 0.0, 1.0).unwrap_err().contains("strictly between"));
    }

    #[test]
    fn binomial_pmf_matches_the_coin_flip_table_to_the_last_digit() {
        // 252 / 1024, a dyadic rational the log-space route must still nail.
        assert!((binomial_pmf(5, 10, 0.5).expect("in range") - 0.24609375).abs() < 1e-12);
        assert!((binomial_pmf(0, 10, 0.5).expect("in range") - 1.0 / 1024.0).abs() < 1e-15);
        assert!((binomial_pmf(3, 5, 0.2).expect("in range") - 0.0512).abs() < 1e-12);
        // Certain and impossible probabilities collapse to single outcomes.
        assert!(close(binomial_pmf(0, 10, 0.0).expect("in range"), 1.0));
        assert!(close(binomial_pmf(3, 10, 0.0).expect("in range"), 0.0));
        assert!(close(binomial_pmf(10, 10, 1.0).expect("in range"), 1.0));
        assert!(close(binomial_pmf(9, 10, 1.0).expect("in range"), 0.0));
    }

    #[test]
    fn binomial_pmf_survives_a_thousand_trials_in_log_space() {
        let central = binomial_pmf(500, 1000, 0.5).expect("in range");
        assert!(central.is_finite() && central > 0.0);
        // Stirling puts the central mass at sqrt(2 / (pi n)) to a few parts
        // in ten thousand.
        let stirling = (2.0 / (std::f64::consts::PI * 1000.0)).sqrt();
        assert!((central - stirling).abs() / stirling < 1e-3);
    }

    #[test]
    fn binomial_cdf_sums_the_masses_and_reaches_one() {
        // 638 / 1024, another dyadic rational.
        assert!((binomial_cdf(5, 10, 0.5).expect("in range") - 0.623046875).abs() < 1e-12);
        assert!(close(binomial_cdf(10, 10, 0.3).expect("in range"), 1.0));
        assert!(close(binomial_cdf(1000, 1000, 0.3).expect("in range"), 1.0));
        assert!(close(binomial_cdf(0, 10, 0.0).expect("in range"), 1.0));
        assert!(close(binomial_cdf(9, 10, 1.0).expect("in range"), 0.0));
    }

    #[test]
    fn the_binomial_functions_reject_every_out_of_range_input() {
        assert!(binomial_pmf(5, -1, 0.5).unwrap_err().contains("cannot be negative"));
        assert!(binomial_pmf(-1, 10, 0.5).unwrap_err().contains("0 to the number of trials"));
        assert!(binomial_pmf(11, 10, 0.5).unwrap_err().contains("0 to the number of trials"));
        assert!(binomial_pmf(5, 10, 1.5).unwrap_err().contains("not a probability"));
        assert!(binomial_pmf(5, 10, -0.1).unwrap_err().contains("not a probability"));
        assert!(binomial_cdf(5, -1, 0.5).unwrap_err().contains("cannot be negative"));
        assert!(binomial_cdf(11, 10, 0.5).unwrap_err().contains("0 to the number of trials"));
        assert!(binomial_cdf(5, 10, 2.0).unwrap_err().contains("not a probability"));
    }

    #[test]
    fn poisson_pmf_matches_the_closed_form() {
        // Two events at rate 3 is 9/2 times e^-3.
        assert!((poisson_pmf(2, 3.0).expect("in range") - 4.5 * (-3.0f64).exp()).abs() < 1e-12);
        assert!((poisson_pmf(0, 3.0).expect("in range") - (-3.0f64).exp()).abs() < 1e-15);
        // A large count at a small rate underflows gracefully instead of
        // blowing up the factorial.
        let far_tail = poisson_pmf(500, 2.0).expect("in range");
        assert!(far_tail.is_finite() && far_tail >= 0.0);
    }

    #[test]
    fn poisson_cdf_sums_the_masses_and_approaches_one() {
        // At most two events at rate 3 is (1 + 3 + 9/2) times e^-3.
        assert!((poisson_cdf(2, 3.0).expect("in range") - 8.5 * (-3.0f64).exp()).abs() < 1e-12);
        assert!(close(poisson_cdf(100, 3.0).expect("in range"), 1.0));
        assert!(poisson_cdf(0, 3.0).expect("in range") < poisson_cdf(1, 3.0).expect("in range"));
    }

    #[test]
    fn the_poisson_functions_reject_bad_rates_and_negative_counts() {
        assert!(poisson_pmf(2, 0.0).unwrap_err().contains("must be positive"));
        assert!(poisson_pmf(2, -3.0).unwrap_err().contains("must be positive"));
        assert!(poisson_pmf(-1, 3.0).unwrap_err().contains("cannot be negative"));
        assert!(poisson_cdf(2, 0.0).unwrap_err().contains("must be positive"));
        assert!(poisson_cdf(-1, 3.0).unwrap_err().contains("cannot be negative"));
    }

    #[test]
    fn sample_size_gives_the_number_every_pollster_quotes() {
        assert_eq!(sample_size_for_proportion(0.03, 0.95).expect("in range"), 1068);
        assert_eq!(sample_size_for_proportion(0.05, 0.95).expect("in range"), 385);
        assert_eq!(sample_size_for_proportion(0.01, 0.99).expect("in range"), 16588);
        // A wider margin can never ask for more people.
        let tight = sample_size_for_proportion(0.02, 0.95).expect("in range");
        let loose = sample_size_for_proportion(0.04, 0.95).expect("in range");
        assert!(loose < tight);
    }

    #[test]
    fn sample_size_rejects_margins_and_confidences_outside_the_open_interval() {
        assert!(sample_size_for_proportion(0.0, 0.95).unwrap_err().contains("margin of error"));
        assert!(sample_size_for_proportion(1.0, 0.95).unwrap_err().contains("margin of error"));
        assert!(sample_size_for_proportion(-0.03, 0.95).unwrap_err().contains("margin of error"));
        assert!(sample_size_for_proportion(0.03, 0.0).unwrap_err().contains("confidence level"));
        assert!(sample_size_for_proportion(0.03, 1.0).unwrap_err().contains("confidence level"));
        assert!(sample_size_for_proportion(0.03, 1.5).unwrap_err().contains("confidence level"));
    }
}
