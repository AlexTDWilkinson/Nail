//! Summary statistics over an array of numbers.
//!
//! The handful of questions anyone asks of a column of data: what is the
//! typical value, how spread out is it, where does the ninety-fifth percentile
//! sit, do these two columns move together. Every one of them is undefined on
//! an empty array, so every one returns a result rather than a number invented
//! out of nothing.

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
}
