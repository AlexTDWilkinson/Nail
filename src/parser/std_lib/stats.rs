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
    let mut sorted = values.clone();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
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

    let mut sorted = values.clone();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
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

    let mut sorted = values.clone();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

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
}
