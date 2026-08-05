//! Learning from data: the handful of models that are worth having built in.
//!
//! Everything here is written out longhand rather than wrapped around a
//! machine-learning framework, for the same reason the rest of this library
//! is: a language with no package manager has to ship what people need, and
//! what people need here is small. A decision tree, a line of best fit, a way
//! to group points that are near each other, and the numbers that say whether
//! any of it worked.
//!
//! Two rules hold throughout. Every function that involves randomness takes
//! the seed as an argument, so a result is reproducible - a model that trains
//! differently every run cannot be debugged. And a model is data: a fitted
//! tree or line is a plain struct you can print, store as JSON, and predict
//! with later, not a handle to something living inside the library.
//!
//! Features are given as an array of rows, each row an array of numbers of
//! the same length. Labels for classification are whole numbers, one per row.

use serde::{Deserialize, Serialize};

/// A dataset cut into a part to learn from and a part to be judged on.
///
/// The split is what stops a model being graded on the answers it memorised.
/// A tree deep enough will predict its training data perfectly and predict
/// nothing else at all, and only the test half will tell you that.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ML_Split {
    pub train_features: Vec<Vec<f64>>,
    pub train_labels: Vec<i64>,
    pub test_features: Vec<Vec<f64>>,
    pub test_labels: Vec<i64>,
}

/// A fitted straight line through the data: one weight per feature, plus the
/// value where every feature is zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ML_Linear {
    pub weights: Vec<f64>,
    pub intercept: f64,
}

/// A fitted decision tree, held as a flat array of nodes rather than nodes
/// pointing at nodes, because that is a shape Nail can carry, print and store.
///
/// Node 0 is the root. For a branch, `feature` is which column to test and
/// `threshold` the value to test it against - not greater than goes to
/// `left`, greater goes to `right`. For a leaf, `feature` is -1, both children
/// are -1, and `prediction` is the answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ML_Tree {
    pub feature: Vec<i64>,
    pub threshold: Vec<f64>,
    pub left: Vec<i64>,
    pub right: Vec<i64>,
    pub prediction: Vec<i64>,
}

/// Points grouped by nearness: where each group's middle ended up, and which
/// group each point was put in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ML_Clusters {
    pub centroids: Vec<Vec<f64>>,
    pub assignments: Vec<i64>,
}

/// How a set of predictions did, counted four ways. Named for the positive
/// label being 1 and everything else being negative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ML_Scores {
    pub true_positive: i64,
    pub false_positive: i64,
    pub true_negative: i64,
    pub false_negative: i64,
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

/// Checks that a feature table is usable: rectangular, non-empty, and as long
/// as its label column.
fn check_dataset(function: &str, features: &Vec<Vec<f64>>, labels_len: usize) -> Result<usize, String> {
    if features.is_empty() {
        return Err(format!("{}: the feature table is empty, so there is nothing to learn from", function));
    }
    if features.len() != labels_len {
        return Err(format!("{}: there are {} rows of features but {} labels", function, features.len(), labels_len));
    }

    let width = features[0].len();
    if width == 0 {
        return Err(format!("{}: the rows have no columns in them", function));
    }
    for (index, row) in features.iter().enumerate() {
        if row.len() != width {
            return Err(format!("{}: row 0 has {} columns but row {} has {}", function, width, index, row.len()));
        }
    }
    return Ok(width);
}

/// A small, fast, entirely predictable generator. Not for anything that needs
/// to be unguessable - see `crypto_random_hex` - but exactly right here, where
/// the same seed must give the same model on every machine forever.
struct Shuffler {
    state: u64,
}

impl Shuffler {
    fn new(seed: i64) -> Shuffler {
        // Zero is a fixed point of this generator, so it is moved off it.
        return Shuffler { state: (seed as u64) ^ 0x9e3779b97f4a7c15 };
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        return self.state;
    }

    fn below(&mut self, limit: usize) -> usize {
        return (self.next() % limit as u64) as usize;
    }
}

/// Cuts a dataset into a part to learn from and a part to be judged on,
/// shuffling first so an ordering in the file does not become an ordering in
/// the split. `train_share` is the fraction that goes to training, so 0.8
/// keeps a fifth back.
pub fn split_train_test(features: Vec<Vec<f64>>, labels: Vec<i64>, train_share: f64, seed: i64) -> Result<ML_Split, String> {
    check_dataset("ml_split_train_test", &features, labels.len())?;
    if !(0.0..=1.0).contains(&train_share) {
        return Err(format!("ml_split_train_test: {} is not a share between 0.0 and 1.0", train_share));
    }

    let mut order: Vec<usize> = (0..features.len()).collect();
    let mut shuffler = Shuffler::new(seed);
    let mut position = order.len();
    while position > 1 {
        position -= 1;
        let swap_with = shuffler.below(position + 1);
        order.swap(position, swap_with);
    }

    let train_count = (train_share * features.len() as f64).round() as usize;
    let mut split = ML_Split { train_features: Vec::new(), train_labels: Vec::new(), test_features: Vec::new(), test_labels: Vec::new() };
    for (place, row_index) in order.iter().enumerate() {
        if place < train_count {
            split.train_features.push(features[*row_index].clone());
            split.train_labels.push(labels[*row_index]);
        } else {
            split.test_features.push(features[*row_index].clone());
            split.test_labels.push(labels[*row_index]);
        }
    }
    return Ok(split);
}

/// Rescales values so the smallest becomes 0.0 and the largest 1.0. What to do
/// before any model that measures distance, so a column in millions does not
/// drown out a column in single digits purely by being written in bigger
/// units. A column that never changes comes back as all zeros.
pub fn normalize(values: Vec<f64>) -> Result<Vec<f64>, String> {
    if values.is_empty() {
        return Err("ml_normalize: the array is empty, so there is nothing to rescale".to_string());
    }
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
    if low == high {
        return Ok(values.iter().map(|_| 0.0).collect());
    }
    return Ok(values.iter().map(|value| (value - low) / (high - low)).collect());
}

/// Rescales values to sit around zero with a spread of one - the other way of
/// putting columns on the same footing, and the one to use when outliers
/// matter, since it does not let a single extreme value squash everything else
/// into a corner.
pub fn standardize(values: Vec<f64>) -> Result<Vec<f64>, String> {
    if values.len() < 2 {
        return Err(format!("ml_standardize: needs at least two values to measure a spread, got {}", values.len()));
    }
    let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
    let variance: f64 = values.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    if variance == 0.0 {
        return Ok(values.iter().map(|_| 0.0).collect());
    }
    let spread = variance.sqrt();
    return Ok(values.iter().map(|value| (value - mean) / spread).collect());
}

/// Solves a system of linear equations by Gaussian elimination with partial
/// pivoting. Used for the exact least-squares fit below.
fn solve(mut matrix: Vec<Vec<f64>>, mut right: Vec<f64>) -> Option<Vec<f64>> {
    let size = right.len();
    for column in 0..size {
        // Pivot on the largest remaining value in this column, which keeps the
        // arithmetic stable when the leading value is near zero.
        let mut best = column;
        for row in column + 1..size {
            if matrix[row][column].abs() > matrix[best][column].abs() {
                best = row;
            }
        }
        if matrix[best][column].abs() < 1e-12 {
            return None;
        }
        matrix.swap(column, best);
        right.swap(column, best);

        for row in column + 1..size {
            let factor = matrix[row][column] / matrix[column][column];
            for inner in column..size {
                matrix[row][inner] -= factor * matrix[column][inner];
            }
            right[row] -= factor * right[column];
        }
    }

    let mut answer = vec![0.0; size];
    for row in (0..size).rev() {
        let mut total = right[row];
        for column in row + 1..size {
            total -= matrix[row][column] * answer[column];
        }
        answer[row] = total / matrix[row][row];
    }
    return Some(answer);
}

/// Fits the straight line that comes closest to the data, in the ordinary
/// least-squares sense. Exact rather than iterative, so there is no learning
/// rate to tune and no question of whether it has converged.
///
/// Fails when the columns say the same thing twice - two features that are
/// copies of each other, or one that is another times a constant - because
/// then infinitely many lines fit equally well and none of them means
/// anything.
pub fn linear_fit(features: Vec<Vec<f64>>, targets: Vec<f64>) -> Result<ML_Linear, String> {
    let width = check_dataset("ml_linear_fit", &features, targets.len())?;
    if features.len() <= width {
        return Err(format!("ml_linear_fit: {} rows cannot pin down {} weights and an intercept", features.len(), width));
    }

    // Normal equations, with a column of ones for the intercept.
    let size = width + 1;
    let mut matrix = vec![vec![0.0; size]; size];
    let mut right = vec![0.0; size];

    for (row_index, row) in features.iter().enumerate() {
        let mut extended = Vec::with_capacity(size);
        extended.push(1.0);
        extended.extend_from_slice(row);

        for i in 0..size {
            for j in 0..size {
                matrix[i][j] += extended[i] * extended[j];
            }
            right[i] += extended[i] * targets[row_index];
        }
    }

    let solution = solve(matrix, right).ok_or_else(|| "ml_linear_fit: the columns repeat each other, so no single line fits best".to_string())?;
    return Ok(ML_Linear { intercept: solution[0], weights: solution[1..].to_vec() });
}

/// What the fitted line says about one row.
pub fn linear_predict(model: &ML_Linear, row: Vec<f64>) -> Result<f64, String> {
    if row.len() != model.weights.len() {
        return Err(format!("ml_linear_predict: the model was fitted on {} columns but this row has {}", model.weights.len(), row.len()));
    }
    let mut total = model.intercept;
    for index in 0..row.len() {
        total += model.weights[index] * row[index];
    }
    return Ok(total);
}

/// How impure a set of labels is, by the Gini measure: 0 when they all agree,
/// approaching 1 the more evenly they are spread across classes.
fn impurity(labels: &[i64]) -> f64 {
    if labels.is_empty() {
        return 0.0;
    }
    let mut classes: Vec<(i64, usize)> = Vec::new();
    for label in labels.iter() {
        match classes.iter_mut().find(|(class, _)| class == label) {
            Some((_, count)) => *count += 1,
            None => classes.push((*label, 1)),
        }
    }
    let total = labels.len() as f64;
    let mut score = 1.0;
    for (_, count) in classes.iter() {
        let share = *count as f64 / total;
        score -= share * share;
    }
    return score;
}

/// The label appearing most often, ties broken by the smaller label so the
/// answer does not depend on the order rows happened to arrive in.
fn majority(labels: &[i64]) -> i64 {
    let mut best = 0;
    let mut best_count = 0;
    let mut seen: Vec<(i64, usize)> = Vec::new();
    for label in labels.iter() {
        match seen.iter_mut().find(|(class, _)| class == label) {
            Some((_, count)) => *count += 1,
            None => seen.push((*label, 1)),
        }
    }
    seen.sort_by_key(|(class, _)| *class);
    for (class, count) in seen.iter() {
        if *count > best_count {
            best = *class;
            best_count = *count;
        }
    }
    return best;
}

/// Grows one node and everything under it, returning its index.
fn grow(tree: &mut ML_Tree, rows: &Vec<Vec<f64>>, labels: &Vec<i64>, indices: Vec<usize>, depth: i64, max_depth: i64, min_rows: usize) -> usize {
    let here = tree.feature.len();
    tree.feature.push(-1);
    tree.threshold.push(0.0);
    tree.left.push(-1);
    tree.right.push(-1);

    let here_labels: Vec<i64> = indices.iter().map(|index| labels[*index]).collect();
    tree.prediction.push(majority(&here_labels));

    if depth >= max_depth || indices.len() < min_rows || impurity(&here_labels) == 0.0 {
        return here;
    }

    // The best split is the one that leaves the two sides purest, weighted by
    // how many rows land on each side.
    let width = rows[0].len();
    let mut best_gain = 0.0;
    let mut best_feature = -1i64;
    let mut best_threshold = 0.0;
    let starting_impurity = impurity(&here_labels);

    for feature in 0..width {
        let mut candidates: Vec<f64> = indices.iter().map(|index| rows[*index][feature]).collect();
        candidates.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        candidates.dedup();
        if candidates.len() < 2 {
            continue;
        }

        // Split halfway between neighbouring values, so the threshold does not
        // sit exactly on a value that appears in the data.
        for pair in candidates.windows(2) {
            let threshold = (pair[0] + pair[1]) / 2.0;
            let mut left_labels = Vec::new();
            let mut right_labels = Vec::new();
            for index in indices.iter() {
                if rows[*index][feature] <= threshold {
                    left_labels.push(labels[*index]);
                } else {
                    right_labels.push(labels[*index]);
                }
            }
            if left_labels.is_empty() || right_labels.is_empty() {
                continue;
            }

            let total = indices.len() as f64;
            let after = (left_labels.len() as f64 / total) * impurity(&left_labels) + (right_labels.len() as f64 / total) * impurity(&right_labels);
            let gain = starting_impurity - after;
            if gain > best_gain + 1e-12 {
                best_gain = gain;
                best_feature = feature as i64;
                best_threshold = threshold;
            }
        }
    }

    if best_feature < 0 {
        return here;
    }

    let mut left_indices = Vec::new();
    let mut right_indices = Vec::new();
    for index in indices.iter() {
        if rows[*index][best_feature as usize] <= best_threshold {
            left_indices.push(*index);
        } else {
            right_indices.push(*index);
        }
    }

    let left_child = grow(tree, rows, labels, left_indices, depth + 1, max_depth, min_rows);
    let right_child = grow(tree, rows, labels, right_indices, depth + 1, max_depth, min_rows);

    tree.feature[here] = best_feature;
    tree.threshold[here] = best_threshold;
    tree.left[here] = left_child as i64;
    tree.right[here] = right_child as i64;
    return here;
}

/// Fits a decision tree by repeatedly splitting on whichever column separates
/// the classes best.
///
/// `max_depth` is what stands between a useful model and one that has
/// memorised the training set: a tree allowed to grow without limit will keep
/// splitting until every leaf holds a single row, score perfectly on the data
/// it saw, and be worthless on anything else. Three to five is a sensible
/// start.
pub fn tree_fit(features: Vec<Vec<f64>>, labels: Vec<i64>, max_depth: i64) -> Result<ML_Tree, String> {
    check_dataset("ml_tree_fit", &features, labels.len())?;
    if max_depth < 1 {
        return Err(format!("ml_tree_fit: a maximum depth of {} leaves no tree to grow", max_depth));
    }
    // A single tree has no way to learn what an absent value means, and
    // silently sending every gap the same way would be a decision made on the
    // program's behalf. ml_boost_fit does learn it, per split.
    for (index, row) in features.iter().enumerate() {
        if row.iter().any(|value| value.is_nan()) {
            return Err(format!("ml_tree_fit: row {} has a column with no value in it - fill the gaps first, or use ml_boost_fit, which learns which way missing rows should go", index));
        }
    }

    let mut tree = ML_Tree { feature: Vec::new(), threshold: Vec::new(), left: Vec::new(), right: Vec::new(), prediction: Vec::new() };
    let indices: Vec<usize> = (0..features.len()).collect();
    grow(&mut tree, &features, &labels, indices, 0, max_depth, 2);
    return Ok(tree);
}

/// What the tree says about one row.
pub fn tree_predict(tree: &ML_Tree, row: Vec<f64>) -> Result<i64, String> {
    if tree.feature.is_empty() {
        return Err("ml_tree_predict: this tree has no nodes in it".to_string());
    }

    let mut at = 0usize;
    // A tree cannot be deeper than it has nodes, so this bounds the walk even
    // if a hand-made tree were to point at itself.
    for _ in 0..tree.feature.len() + 1 {
        let feature = tree.feature[at];
        if feature < 0 {
            return Ok(tree.prediction[at]);
        }
        if feature as usize >= row.len() {
            return Err(format!("ml_tree_predict: the tree tests column {} but this row has {} columns", feature, row.len()));
        }
        at = if row[feature as usize] <= tree.threshold[at] { tree.left[at] as usize } else { tree.right[at] as usize };
    }
    return Err("ml_tree_predict: this tree loops back on itself".to_string());
}

/// Writes the tree out as the rules it actually applies.
///
/// This is the reason to reach for a tree over something more accurate: you
/// can read it. `feature_names` gives the columns their real names; pass an
/// empty array to see them numbered.
pub fn tree_explain(tree: &ML_Tree, feature_names: Vec<String>) -> Result<String, String> {
    if tree.feature.is_empty() {
        return Err("ml_tree_explain: this tree has no nodes in it".to_string());
    }

    fn name_of(names: &Vec<String>, index: i64) -> String {
        return match names.get(index as usize) {
            Some(name) => name.clone(),
            None => format!("column {}", index),
        };
    }

    fn describe(tree: &ML_Tree, names: &Vec<String>, at: usize, depth: usize, out: &mut String) {
        let padding = "  ".repeat(depth);
        if tree.feature[at] < 0 {
            out.push_str(&format!("{}answer {}\n", padding, tree.prediction[at]));
            return;
        }
        let column = name_of(names, tree.feature[at]);
        out.push_str(&format!("{}if {} <= {}\n", padding, column, tree.threshold[at]));
        describe(tree, names, tree.left[at] as usize, depth + 1, out);
        out.push_str(&format!("{}else\n", padding));
        describe(tree, names, tree.right[at] as usize, depth + 1, out);
    }

    let mut out = String::new();
    describe(tree, &feature_names, 0, 0, &mut out);
    return Ok(out);
}

/// The straight-line distance between two rows.
fn distance(left: &[f64], right: &[f64]) -> f64 {
    let mut total = 0.0;
    for index in 0..left.len() {
        let gap = left[index] - right[index];
        total += gap * gap;
    }
    return total.sqrt();
}

/// Predicts a label by asking the `k` nearest rows what they are, and taking
/// the answer most of them give. No fitting happens - the training data is the
/// model - which makes this the thing to reach for when there is very little
/// data to learn from.
///
/// Distances are raw, so rescale the columns with `ml_normalize` first unless
/// they are already in comparable units.
pub fn knn_predict(features: Vec<Vec<f64>>, labels: Vec<i64>, query: Vec<f64>, k: i64) -> Result<i64, String> {
    let width = check_dataset("ml_knn_predict", &features, labels.len())?;
    if query.len() != width {
        return Err(format!("ml_knn_predict: the data has {} columns but the row asked about has {}", width, query.len()));
    }
    if k < 1 {
        return Err(format!("ml_knn_predict: asked for the {} nearest rows, which is not a count", k));
    }
    if k as usize > features.len() {
        return Err(format!("ml_knn_predict: asked for the {} nearest rows out of {}", k, features.len()));
    }

    let mut distances: Vec<(f64, i64)> = features.iter().enumerate().map(|(index, row)| (distance(row, &query), labels[index])).collect();
    // Ties broken by label, so the answer never depends on row order.
    distances.sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap_or(std::cmp::Ordering::Equal).then(left.1.cmp(&right.1)));

    let nearest: Vec<i64> = distances.iter().take(k as usize).map(|(_, label)| *label).collect();
    return Ok(majority(&nearest));
}

/// Groups points by nearness, into `k` groups, by Lloyd's algorithm: start
/// from `k` of the points themselves, put every point with the nearest middle,
/// move each middle to the average of what it caught, and repeat.
///
/// The starting points are chosen by the seed, and the answer depends on them,
/// which is why the seed is an argument rather than a hidden decision. An
/// empty group is refilled from the point furthest from its own middle, so k
/// groups always come back.
pub fn kmeans(points: Vec<Vec<f64>>, k: i64, seed: i64, iterations: i64) -> Result<ML_Clusters, String> {
    let width = check_dataset("ml_kmeans", &points, points.len())?;
    if k < 1 {
        return Err(format!("ml_kmeans: {} groups is not a count", k));
    }
    if k as usize > points.len() {
        return Err(format!("ml_kmeans: asked for {} groups from {} points", k, points.len()));
    }
    if iterations < 1 {
        return Err(format!("ml_kmeans: {} rounds leaves nothing to do", iterations));
    }

    let mut shuffler = Shuffler::new(seed);
    let mut chosen: Vec<usize> = Vec::new();
    while chosen.len() < k as usize {
        let candidate = shuffler.below(points.len());
        if !chosen.contains(&candidate) {
            chosen.push(candidate);
        }
    }
    let mut centroids: Vec<Vec<f64>> = chosen.iter().map(|index| points[*index].clone()).collect();
    let mut assignments: Vec<i64> = vec![0; points.len()];

    for _ in 0..iterations {
        let mut moved = false;
        for (index, point) in points.iter().enumerate() {
            let mut best = 0;
            let mut best_distance = distance(point, &centroids[0]);
            for group in 1..centroids.len() {
                let candidate = distance(point, &centroids[group]);
                if candidate < best_distance {
                    best = group;
                    best_distance = candidate;
                }
            }
            if assignments[index] != best as i64 {
                assignments[index] = best as i64;
                moved = true;
            }
        }

        for group in 0..centroids.len() {
            let members: Vec<&Vec<f64>> = points.iter().enumerate().filter(|(index, _)| assignments[*index] == group as i64).map(|(_, point)| point).collect();
            if members.is_empty() {
                // Refill from the point that fits its own group worst, which
                // is the point most in need of a group of its own.
                let mut worst = 0;
                let mut worst_distance = -1.0;
                for (index, point) in points.iter().enumerate() {
                    let own = distance(point, &centroids[assignments[index] as usize]);
                    if own > worst_distance {
                        worst = index;
                        worst_distance = own;
                    }
                }
                centroids[group] = points[worst].clone();
                assignments[worst] = group as i64;
                continue;
            }

            let mut middle = vec![0.0; width];
            for member in members.iter() {
                for column in 0..width {
                    middle[column] += member[column];
                }
            }
            for column in 0..width {
                middle[column] /= members.len() as f64;
            }
            centroids[group] = middle;
        }

        if !moved {
            break;
        }
    }

    return Ok(ML_Clusters { centroids, assignments });
}

/// Counts how a set of predictions did, treating the label 1 as positive and
/// everything else as negative.
///
/// Accuracy on its own is a trap: on data where one case in a thousand is
/// positive, a model answering "negative" every single time scores 99.9%.
/// Precision says how often a positive answer was right, recall how many of
/// the real positives were found, and f1 balances the two - which is why all
/// four come back together rather than one at a time.
pub fn score(predicted: Vec<i64>, actual: Vec<i64>) -> Result<ML_Scores, String> {
    if predicted.len() != actual.len() {
        return Err(format!("ml_score: {} predictions against {} real labels", predicted.len(), actual.len()));
    }
    if predicted.is_empty() {
        return Err("ml_score: there are no predictions to score".to_string());
    }

    let mut true_positive = 0i64;
    let mut false_positive = 0i64;
    let mut true_negative = 0i64;
    let mut false_negative = 0i64;

    for index in 0..predicted.len() {
        let said_yes = predicted[index] == 1;
        let was_yes = actual[index] == 1;
        match (said_yes, was_yes) {
            (true, true) => true_positive += 1,
            (true, false) => false_positive += 1,
            (false, false) => true_negative += 1,
            (false, true) => false_negative += 1,
        }
    }

    let total = predicted.len() as f64;
    let accuracy = (true_positive + true_negative) as f64 / total;

    // A model that never says yes has no precision to speak of, and one with
    // nothing to find has no recall. Zero is the honest answer for both.
    let precision = if true_positive + false_positive == 0 { 0.0 } else { true_positive as f64 / (true_positive + false_positive) as f64 };
    let recall = if true_positive + false_negative == 0 { 0.0 } else { true_positive as f64 / (true_positive + false_negative) as f64 };
    let f1 = if precision + recall == 0.0 { 0.0 } else { 2.0 * precision * recall / (precision + recall) };

    return Ok(ML_Scores { true_positive, false_positive, true_negative, false_negative, accuracy, precision, recall, f1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 1e-6;
    }

    /// A dataset a tree can separate exactly: label 1 when the first column is
    /// above 5, label 0 otherwise, with a second column that says nothing.
    fn separable() -> (Vec<Vec<f64>>, Vec<i64>) {
        let features = vec![
            vec![1.0, 3.0],
            vec![2.0, 9.0],
            vec![3.0, 1.0],
            vec![4.0, 7.0],
            vec![6.0, 2.0],
            vec![7.0, 8.0],
            vec![8.0, 4.0],
            vec![9.0, 6.0],
        ];
        let labels = vec![0, 0, 0, 0, 1, 1, 1, 1];
        return (features, labels);
    }

    #[test]
    fn a_dataset_that_is_not_rectangular_is_refused() {
        let ragged = vec![vec![1.0, 2.0], vec![3.0]];
        assert!(tree_fit(ragged, vec![0, 1], 3).unwrap_err().contains("row 0 has 2 columns but row 1 has 1"));
        assert!(tree_fit(vec![], vec![], 3).unwrap_err().contains("empty"));
        assert!(tree_fit(vec![vec![1.0]], vec![0, 1], 3).unwrap_err().contains("1 rows of features but 2 labels"));
    }

    #[test]
    fn splitting_keeps_every_row_and_answers_the_same_way_for_a_seed() {
        let (features, labels) = separable();
        let split = split_train_test(features.clone(), labels.clone(), 0.75, 42).expect("a valid share");
        assert_eq!(split.train_features.len(), 6);
        assert_eq!(split.test_features.len(), 2);
        assert_eq!(split.train_labels.len(), 6);
        assert_eq!(split.test_labels.len(), 2);

        let again = split_train_test(features, labels, 0.75, 42).expect("a valid share");
        assert_eq!(split.train_labels, again.train_labels, "the same seed must give the same split");
    }

    #[test]
    fn splitting_shuffles_rather_than_taking_the_first_rows() {
        // The labels are sorted, so an unshuffled split would put every 0 in
        // training and every 1 in test.
        let (features, labels) = separable();
        let split = split_train_test(features, labels, 0.75, 7).expect("a valid share");
        assert!(split.train_labels.contains(&1), "the training half must not be all one class: {:?}", split.train_labels);
    }

    #[test]
    fn rescaling_puts_columns_on_the_same_footing() {
        let scaled = normalize(vec![10.0, 20.0, 30.0]).expect("values");
        assert!(close(scaled[0], 0.0) && close(scaled[1], 0.5) && close(scaled[2], 1.0));

        // A column that never changes has no range to rescale into.
        assert_eq!(normalize(vec![5.0, 5.0]).expect("values"), vec![0.0, 0.0]);
        assert!(normalize(vec![]).unwrap_err().contains("empty"));

        let standardized = standardize(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]).expect("values");
        let mean: f64 = standardized.iter().sum::<f64>() / standardized.len() as f64;
        assert!(close(mean, 0.0), "standardized values sit around zero, got {}", mean);
        assert!(standardize(vec![1.0]).unwrap_err().contains("at least two values"));
    }

    #[test]
    fn a_line_is_fitted_exactly_when_the_data_is_a_line() {
        // y = 2a + 3b + 1
        let features = vec![vec![1.0, 1.0], vec![2.0, 1.0], vec![1.0, 2.0], vec![3.0, 5.0], vec![4.0, 2.0]];
        let targets: Vec<f64> = features.iter().map(|row| 2.0 * row[0] + 3.0 * row[1] + 1.0).collect();

        let model = linear_fit(features, targets).expect("independent columns");
        assert!(close(model.weights[0], 2.0), "got {:?}", model);
        assert!(close(model.weights[1], 3.0), "got {:?}", model);
        assert!(close(model.intercept, 1.0), "got {:?}", model);
        assert!(close(linear_predict(&model, vec![10.0, 10.0]).expect("the right width"), 51.0));
    }

    #[test]
    fn a_line_cannot_be_fitted_through_columns_that_repeat_each_other() {
        // The second column is the first doubled, so no single answer exists.
        let features = vec![vec![1.0, 2.0], vec![2.0, 4.0], vec![3.0, 6.0], vec![4.0, 8.0]];
        let targets = vec![1.0, 2.0, 3.0, 4.0];
        assert!(linear_fit(features, targets).unwrap_err().contains("columns repeat each other"));
    }

    #[test]
    fn a_line_needs_more_rows_than_it_has_weights() {
        let features = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
        assert!(linear_fit(features, vec![1.0, 2.0]).unwrap_err().contains("cannot pin down"));
    }

    #[test]
    fn predicting_with_the_wrong_number_of_columns_is_an_error() {
        let model = ML_Linear { weights: vec![1.0, 2.0], intercept: 0.0 };
        assert!(linear_predict(&model, vec![1.0]).unwrap_err().contains("fitted on 2 columns but this row has 1"));
    }

    #[test]
    fn a_tree_learns_the_rule_that_separates_the_classes() {
        let (features, labels) = separable();
        let tree = tree_fit(features.clone(), labels.clone(), 3).expect("a valid depth");

        for (index, row) in features.iter().enumerate() {
            assert_eq!(tree_predict(&tree, row.clone()).expect("the right width"), labels[index], "row {:?} was misread", row);
        }

        // And it generalises to rows it never saw, because the rule is real.
        assert_eq!(tree_predict(&tree, vec![0.5, 5.0]).expect("the right width"), 0);
        assert_eq!(tree_predict(&tree, vec![9.5, 5.0]).expect("the right width"), 1);
    }

    #[test]
    fn a_tree_splits_on_the_column_that_matters_and_ignores_the_one_that_does_not() {
        let (features, labels) = separable();
        let tree = tree_fit(features, labels, 3).expect("a valid depth");
        assert_eq!(tree.feature[0], 0, "the root must split on the column that separates the classes");
        assert!(tree.threshold[0] > 4.0 && tree.threshold[0] < 6.0, "the split sits between the classes, got {}", tree.threshold[0]);
    }

    #[test]
    fn a_tree_can_be_read_as_the_rules_it_applies() {
        let (features, labels) = separable();
        let tree = tree_fit(features, labels, 2).expect("a valid depth");

        let named = tree_explain(&tree, vec!["size".to_string(), "noise".to_string()]).expect("a grown tree");
        assert!(named.contains("if size <="), "got:\n{}", named);
        assert!(named.contains("answer 0"), "got:\n{}", named);
        assert!(named.contains("answer 1"), "got:\n{}", named);
        assert!(named.contains("else"), "got:\n{}", named);

        // With no names, the columns are numbered rather than left blank.
        let numbered = tree_explain(&tree, vec![]).expect("a grown tree");
        assert!(numbered.contains("if column 0 <="), "got:\n{}", numbered);
    }

    #[test]
    fn a_tree_of_one_class_is_a_single_leaf() {
        let tree = tree_fit(vec![vec![1.0], vec![2.0], vec![3.0]], vec![1, 1, 1], 3).expect("a valid depth");
        assert_eq!(tree.feature.len(), 1, "nothing to split on when every label agrees");
        assert_eq!(tree.feature[0], -1);
        assert_eq!(tree_predict(&tree, vec![99.0]).expect("the right width"), 1);
    }

    #[test]
    fn a_tree_needs_somewhere_to_grow() {
        assert!(tree_fit(vec![vec![1.0]], vec![0], 0).unwrap_err().contains("leaves no tree to grow"));
        let empty = ML_Tree { feature: vec![], threshold: vec![], left: vec![], right: vec![], prediction: vec![] };
        assert!(tree_predict(&empty, vec![1.0]).unwrap_err().contains("no nodes"));
        assert!(tree_explain(&empty, vec![]).unwrap_err().contains("no nodes"));
    }

    #[test]
    fn nearest_neighbours_answer_with_what_is_around_them() {
        let (features, labels) = separable();
        assert_eq!(knn_predict(features.clone(), labels.clone(), vec![1.5, 3.0], 3).expect("valid"), 0);
        assert_eq!(knn_predict(features.clone(), labels.clone(), vec![8.5, 5.0], 3).expect("valid"), 1);
        // One neighbour is the nearest row itself.
        assert_eq!(knn_predict(features.clone(), labels.clone(), vec![1.0, 3.0], 1).expect("valid"), 0);
    }

    #[test]
    fn nearest_neighbours_refuses_counts_it_cannot_honour() {
        let (features, labels) = separable();
        assert!(knn_predict(features.clone(), labels.clone(), vec![1.0, 1.0], 0).unwrap_err().contains("not a count"));
        assert!(knn_predict(features.clone(), labels.clone(), vec![1.0, 1.0], 99).unwrap_err().contains("out of 8"));
        assert!(knn_predict(features, labels, vec![1.0], 3).unwrap_err().contains("row asked about has 1"));
    }

    #[test]
    fn grouping_finds_two_clumps_that_are_plainly_two_clumps() {
        let points = vec![
            vec![0.0, 0.0],
            vec![0.5, 0.2],
            vec![0.2, 0.4],
            vec![10.0, 10.0],
            vec![10.5, 9.8],
            vec![9.7, 10.2],
        ];
        let clusters = kmeans(points, 2, 42, 20).expect("valid");
        assert_eq!(clusters.centroids.len(), 2);
        assert_eq!(clusters.assignments.len(), 6);
        assert_eq!(clusters.assignments[0], clusters.assignments[1], "the near points belong together");
        assert_eq!(clusters.assignments[0], clusters.assignments[2]);
        assert_eq!(clusters.assignments[3], clusters.assignments[4], "and so do the far ones");
        assert_ne!(clusters.assignments[0], clusters.assignments[3], "the two clumps are not one clump");
    }

    #[test]
    fn grouping_answers_the_same_way_for_the_same_seed() {
        let points = vec![vec![0.0], vec![1.0], vec![8.0], vec![9.0], vec![20.0], vec![21.0]];
        let first = kmeans(points.clone(), 3, 7, 20).expect("valid");
        let second = kmeans(points, 3, 7, 20).expect("valid");
        assert_eq!(first.assignments, second.assignments);
    }

    #[test]
    fn grouping_refuses_what_it_cannot_do() {
        let points = vec![vec![1.0], vec![2.0]];
        assert!(kmeans(points.clone(), 0, 1, 10).unwrap_err().contains("not a count"));
        assert!(kmeans(points.clone(), 5, 1, 10).unwrap_err().contains("5 groups from 2 points"));
        assert!(kmeans(points, 1, 1, 0).unwrap_err().contains("nothing to do"));
    }

    #[test]
    fn scoring_counts_all_four_ways_a_prediction_can_land() {
        //             yes  yes  no   no   yes  no
        let predicted = vec![1, 1, 0, 0, 1, 0];
        let actual = vec![1, 0, 0, 1, 1, 0];
        let scores = score(predicted, actual).expect("matching lengths");

        assert_eq!(scores.true_positive, 2);
        assert_eq!(scores.false_positive, 1);
        assert_eq!(scores.true_negative, 2);
        assert_eq!(scores.false_negative, 1);
        assert!(close(scores.accuracy, 4.0 / 6.0));
        assert!(close(scores.precision, 2.0 / 3.0));
        assert!(close(scores.recall, 2.0 / 3.0));
        assert!(close(scores.f1, 2.0 / 3.0));
    }

    /// The reason all four numbers come back together.
    #[test]
    fn accuracy_alone_flatters_a_model_that_never_says_yes() {
        let actual: Vec<i64> = (0..1000).map(|index| if index == 0 { 1 } else { 0 }).collect();
        let never_yes: Vec<i64> = actual.iter().map(|_| 0).collect();

        let scores = score(never_yes, actual).expect("matching lengths");
        assert!(scores.accuracy > 0.998, "accuracy looks excellent: {}", scores.accuracy);
        assert_eq!(scores.recall, 0.0, "and yet it found none of the positives");
        assert_eq!(scores.precision, 0.0);
        assert_eq!(scores.f1, 0.0);
    }

    #[test]
    fn scoring_refuses_what_it_cannot_compare() {
        assert!(score(vec![1], vec![1, 0]).unwrap_err().contains("1 predictions against 2 real labels"));
        assert!(score(vec![], vec![]).unwrap_err().contains("no predictions"));
    }

    /// What the whole module is for, end to end: split, fit, predict, score.
    #[test]
    fn a_model_can_be_trained_and_then_judged_on_data_it_never_saw() {
        let mut features = Vec::new();
        let mut labels = Vec::new();
        for index in 0..60 {
            let value = index as f64;
            features.push(vec![value, (index % 7) as f64]);
            labels.push(if value > 30.0 { 1 } else { 0 });
        }

        let split = split_train_test(features, labels, 0.7, 2024).expect("a valid share");
        let tree = tree_fit(split.train_features, split.train_labels, 4).expect("a valid depth");

        let predicted: Vec<i64> = split.test_features.iter().map(|row| tree_predict(&tree, row.clone()).expect("the right width")).collect();
        let scores = score(predicted, split.test_labels).expect("matching lengths");
        assert!(scores.accuracy > 0.9, "the rule is learnable, so held-out accuracy should be high: {}", scores.accuracy);
    }
}

/// What a boosted model is trying to predict.
///
/// `Squared` fits a number - a price, a duration, a count. `Logistic` fits a
/// yes-or-no answer, and is not the same thing as fitting `Squared` against 0
/// and 1: it optimises the odds rather than the distance, so a confident wrong
/// answer is punished far more than a hesitant one, which is what you want
/// when the output is a probability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ML_Objective {
    Squared,
    Logistic,
}

/// How a boosted model is grown. `ml_boost_default_config` gives sensible
/// values for all of it; change one field and leave the rest.
///
/// `early_stopping_rounds` only does anything in `ml_boost_fit_validated`: it
/// is how many trees in a row may fail to improve the held-out score before
/// training gives up and keeps the best model it had.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ML_BoostConfig {
    pub trees: i64,
    pub learning_rate: f64,
    pub max_depth: i64,
    pub min_samples_leaf: i64,
    pub bins: i64,
    pub lambda_l2: f64,
    pub objective: ML_Objective,
    pub early_stopping_rounds: i64,
}

/// A fitted gradient boosting model: many small trees, each correcting what
/// the ones before it got wrong.
///
/// Held as flat arrays for the same reason `ML_Tree` is - a forest of nodes
/// pointing at nodes is not a shape Nail can carry, and this one can be
/// printed, stored as JSON, and loaded back. `roots` holds the first node of
/// each tree, in the order they were grown; every other array is indexed by
/// node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ML_Boost {
    pub base_score: f64,
    pub roots: Vec<i64>,
    pub feature: Vec<i64>,
    pub threshold: Vec<f64>,
    pub left: Vec<i64>,
    pub right: Vec<i64>,
    /// Which way a row goes when the column being tested has no value in it.
    /// Chosen per split by whichever side the missing rows help more, which is
    /// what lets a model learn that an absent value means something.
    pub default_left: Vec<bool>,
    pub value: Vec<f64>,
    pub gain: Vec<f64>,
    pub columns: i64,
    pub objective: ML_Objective,
    /// How many trees were actually kept. Training may stop early, and then
    /// this is smaller than the configured number.
    pub trees_used: i64,
}

/// How a set of predicted numbers did against the real ones.
///
/// Regression is judged differently from classification, and by more than one
/// number on purpose. `r_squared` says how much of the variation the model
/// explains, `rmse` punishes large misses, `mae` treats every miss alike,
/// `mape` puts the miss as a percentage of the truth - which is the one a
/// person asks for ("how far off is it, typically?") - and `median_ape` is
/// that same percentage with the handful of wild misses unable to drag it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ML_Regression {
    pub r_squared: f64,
    pub mae: f64,
    pub rmse: f64,
    pub mape: f64,
    pub median_ape: f64,
    pub within_ten_percent: f64,
}

/// Sensible values to start from: 100 trees, a slow enough learning rate that
/// no single tree dominates, and a depth shallow enough that the model
/// generalises.
pub fn boost_default_config() -> ML_BoostConfig {
    return ML_BoostConfig {
        trees: 100,
        learning_rate: 0.1,
        max_depth: 6,
        min_samples_leaf: 20,
        bins: 255,
        lambda_l2: 1.0,
        objective: ML_Objective::Squared,
        early_stopping_rounds: 10,
    };
}

/// The split points to consider for each column, chosen once at the start.
///
/// This is the idea the whole method is named for. Considering every distinct
/// value of every column as a possible split costs time proportional to the
/// number of rows, on every node of every tree. Choosing a few hundred
/// quantile boundaries up front and only ever splitting there costs almost
/// nothing per node and loses almost no accuracy, because a split halfway
/// between two nearly equal values was never going to be the one that
/// mattered.
fn feature_bins(rows: &Vec<Vec<f64>>, columns: usize, bins: usize) -> Vec<Vec<f64>> {
    let mut all = Vec::with_capacity(columns);
    for column in 0..columns {
        let mut values: Vec<f64> = rows.iter().map(|row| row[column]).collect();
        values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        values.dedup();

        if values.len() <= 1 {
            // A column that never changes has nothing to split on.
            all.push(Vec::new());
            continue;
        }

        if values.len() <= bins {
            let mut thresholds = Vec::with_capacity(values.len() - 1);
            for index in 0..values.len() - 1 {
                thresholds.push((values[index] + values[index + 1]) / 2.0);
            }
            all.push(thresholds);
            continue;
        }

        let mut thresholds = Vec::with_capacity(bins);
        for step in 1..bins {
            let at = (step * values.len()) / bins;
            if at > 0 && at < values.len() {
                thresholds.push(values[at]);
            }
        }
        thresholds.dedup();
        all.push(thresholds);
    }
    return all;
}

/// How good a group of rows is, as one number: the squared sum of gradients
/// over the sum of hessians, damped by the L2 term. A split is worth making
/// when the two halves score better together than the whole did.
fn node_score(gradient: f64, hessian: f64, lambda_l2: f64) -> f64 {
    if hessian + lambda_l2 <= 0.0 {
        return 0.0;
    }
    return (gradient * gradient) / (hessian + lambda_l2);
}

/// Grows one tree of the boosted model against the current gradients,
/// returning the index of its root node.
fn boost_grow(
    model: &mut ML_Boost,
    rows: &Vec<Vec<f64>>,
    gradients: &Vec<f64>,
    hessians: &Vec<f64>,
    bins: &Vec<Vec<f64>>,
    indices: Vec<usize>,
    depth: i64,
    config: &ML_BoostConfig,
) -> usize {
    let here = model.feature.len();
    model.feature.push(-1);
    model.threshold.push(0.0);
    model.left.push(-1);
    model.right.push(-1);
    model.default_left.push(true);
    model.gain.push(0.0);

    let parent_gradient: f64 = indices.iter().map(|index| gradients[*index]).sum();
    let parent_hessian: f64 = indices.iter().map(|index| hessians[*index]).sum();

    // The leaf value that minimises squared loss for these rows, shrunk by the
    // learning rate so that no single tree moves the answer very far. That
    // shrinking is what makes boosting work: a hundred small corrections
    // generalise where one large one memorises.
    let leaf_value = -config.learning_rate * parent_gradient / (parent_hessian + config.lambda_l2);
    model.value.push(leaf_value);

    if depth >= config.max_depth || (indices.len() as i64) < 2 * config.min_samples_leaf {
        return here;
    }

    let parent_score = node_score(parent_gradient, parent_hessian, config.lambda_l2);
    let mut best_gain = 0.0;
    let mut best_feature = -1i64;
    let mut best_threshold = 0.0;
    let mut best_default_left = true;

    for column in 0..bins.len() {
        // Rows with no value in this column are held aside, and then tried on
        // each side in turn. Which side they help more is a real thing to
        // learn: a missing income and a missing postcode do not mean the same
        // thing, and a model that always sends them the same way cannot tell.
        let mut missing_gradient = 0.0;
        let mut missing_hessian = 0.0;
        let mut missing_count = 0i64;
        for index in indices.iter() {
            if rows[*index][column].is_nan() {
                missing_gradient += gradients[*index];
                missing_hessian += hessians[*index];
                missing_count += 1;
            }
        }

        for threshold in bins[column].iter() {
            let mut present_left_gradient = 0.0;
            let mut present_left_hessian = 0.0;
            let mut present_left_count = 0i64;
            for index in indices.iter() {
                let value = rows[*index][column];
                if !value.is_nan() && value <= *threshold {
                    present_left_gradient += gradients[*index];
                    present_left_hessian += hessians[*index];
                    present_left_count += 1;
                }
            }

            for missing_goes_left in [true, false] {
                let (left_gradient, left_hessian, left_count) = if missing_goes_left {
                    (present_left_gradient + missing_gradient, present_left_hessian + missing_hessian, present_left_count + missing_count)
                } else {
                    (present_left_gradient, present_left_hessian, present_left_count)
                };

                let right_count = indices.len() as i64 - left_count;
                if left_count < config.min_samples_leaf || right_count < config.min_samples_leaf {
                    continue;
                }

                let right_gradient = parent_gradient - left_gradient;
                let right_hessian = parent_hessian - left_hessian;
                let gain = 0.5
                    * (node_score(left_gradient, left_hessian, config.lambda_l2) + node_score(right_gradient, right_hessian, config.lambda_l2) - parent_score);

                if gain > best_gain {
                    best_gain = gain;
                    best_feature = column as i64;
                    best_threshold = *threshold;
                    best_default_left = missing_goes_left;
                }

                // With nothing missing, both trials are the same split.
                if missing_count == 0 {
                    break;
                }
            }
        }
    }

    if best_feature < 0 {
        return here;
    }

    let mut left_indices = Vec::new();
    let mut right_indices = Vec::new();
    for index in indices.iter() {
        let value = rows[*index][best_feature as usize];
        let goes_left = if value.is_nan() { best_default_left } else { value <= best_threshold };
        if goes_left {
            left_indices.push(*index);
        } else {
            right_indices.push(*index);
        }
    }

    let left_child = boost_grow(model, rows, gradients, hessians, bins, left_indices, depth + 1, config);
    let right_child = boost_grow(model, rows, gradients, hessians, bins, right_indices, depth + 1, config);

    model.feature[here] = best_feature;
    model.threshold[here] = best_threshold;
    model.left[here] = left_child as i64;
    model.right[here] = right_child as i64;
    model.default_left[here] = best_default_left;
    model.gain[here] = best_gain;
    return here;
}

/// What one tree of the model says about one row.
fn boost_tree_value(model: &ML_Boost, root: usize, row: &Vec<f64>) -> f64 {
    let mut at = root;
    for _ in 0..model.feature.len() + 1 {
        if model.feature[at] < 0 {
            return model.value[at];
        }
        let value = row[model.feature[at] as usize];
        let goes_left = if value.is_nan() { model.default_left[at] } else { value <= model.threshold[at] };
        at = if goes_left { model.left[at] as usize } else { model.right[at] as usize };
    }
    return 0.0;
}

/// Turns a raw boosted score into a probability. The inverse of the log-odds
/// the logistic objective works in.
fn sigmoid(raw: f64) -> f64 {
    return 1.0 / (1.0 + (-raw).exp());
}

/// The gradient and hessian of the loss at one row - the two numbers a tree is
/// actually fitted to.
///
/// For squared loss the gradient is how far above the truth the model sits and
/// the curvature is constant. For logistic loss the gradient is how far the
/// predicted probability sits above the answer, and the curvature falls away
/// as the model grows confident, which is what stops confident rows from being
/// pushed further and further.
fn loss_derivatives(objective: ML_Objective, raw: f64, target: f64) -> (f64, f64) {
    match objective {
        ML_Objective::Squared => (raw - target, 1.0),
        ML_Objective::Logistic => {
            let probability = sigmoid(raw);
            // Floored, because a hessian of zero would divide the leaf value
            // by the L2 term alone and let one saturated row dominate.
            let curvature = (probability * (1.0 - probability)).max(1e-6);
            (probability - target, curvature)
        }
    }
}

/// Fits a gradient boosting model: start from the average, then grow one small
/// tree at a time, each one trained on what the model still gets wrong.
///
/// This is the method that wins on ordinary tabular data - the kind that comes
/// out of a database with a few dozen columns - and it is what LightGBM and
/// XGBoost implement. The two ideas that make it fast enough to be worth
/// having are both here: split points are chosen once from quantiles rather
/// than searched over every value (see `feature_bins`), and each tree is fitted
/// to the gradient of the loss rather than to the target, so the arithmetic per
/// node is a pair of sums.
///
/// Predicts a number, not a class. For yes-or-no questions, fit against 0 and
/// 1 and treat anything above 0.5 as yes.
pub fn boost_fit(features: Vec<Vec<f64>>, targets: Vec<f64>, config: &ML_BoostConfig) -> Result<ML_Boost, String> {
    return boost_train("ml_boost_fit", features, targets, None, config);
}

/// Fits a boosted model while watching a held-out set, and stops as soon as
/// that set stops improving.
///
/// This is the answer to the only hard question `ml_boost_fit` asks: how many
/// trees? Too few and the model has not finished learning; too many and it
/// starts memorising, and the training score keeps improving the whole time so
/// it cannot tell you which side you are on. Watching data the model is not
/// learning from can. Training stops after `early_stopping_rounds` trees in a
/// row fail to improve the held-out loss, and the trees grown after the best
/// one are thrown away rather than kept.
///
/// The validation rows must not be rows the model is training on, or this
/// measures nothing - use `ml_split_train_test` to get them.
pub fn boost_fit_validated(
    features: Vec<Vec<f64>>,
    targets: Vec<f64>,
    validation_features: Vec<Vec<f64>>,
    validation_targets: Vec<f64>,
    config: &ML_BoostConfig,
) -> Result<ML_Boost, String> {
    check_dataset("ml_boost_fit_validated", &validation_features, validation_targets.len())?;
    if config.early_stopping_rounds < 1 {
        return Err(format!("ml_boost_fit_validated: stopping after {} rounds without improvement is not a rule - use ml_boost_fit to train every tree", config.early_stopping_rounds));
    }
    return boost_train("ml_boost_fit_validated", features, targets, Some((validation_features, validation_targets)), config);
}

/// The training loop both entry points share.
fn boost_train(
    function: &str,
    features: Vec<Vec<f64>>,
    targets: Vec<f64>,
    validation: Option<(Vec<Vec<f64>>, Vec<f64>)>,
    config: &ML_BoostConfig,
) -> Result<ML_Boost, String> {
    let columns = check_dataset(function, &features, targets.len())?;
    if let Some((validation_features, _)) = validation.as_ref() {
        if validation_features[0].len() != columns {
            return Err(format!("{}: the training rows have {} columns but the validation rows have {}", function, columns, validation_features[0].len()));
        }
    }
    if config.objective == ML_Objective::Logistic {
        if let Some(bad) = targets.iter().find(|target| **target != 0.0 && **target != 1.0) {
            return Err(format!("{}: the logistic objective needs every target to be 0 or 1, and one of them is {}", function, bad));
        }
    }
    if config.trees < 1 {
        return Err(format!("ml_boost_fit: {} trees is not a model", config.trees));
    }
    if config.max_depth < 1 {
        return Err(format!("ml_boost_fit: a maximum depth of {} leaves no tree to grow", config.max_depth));
    }
    if config.min_samples_leaf < 1 {
        return Err(format!("ml_boost_fit: {} rows per leaf is not a limit", config.min_samples_leaf));
    }
    if config.bins < 2 {
        return Err(format!("ml_boost_fit: {} bins cannot describe a column", config.bins));
    }
    if config.learning_rate <= 0.0 || config.learning_rate > 1.0 {
        return Err(format!("ml_boost_fit: a learning rate of {} is outside the 0.0 to 1.0 that makes sense", config.learning_rate));
    }
    if config.lambda_l2 < 0.0 {
        return Err(format!("ml_boost_fit: a negative L2 term of {} would reward complexity instead of penalising it", config.lambda_l2));
    }

    let mean: f64 = targets.iter().sum::<f64>() / targets.len() as f64;

    // Where the model starts before any tree has been grown: the average for
    // squared loss, and the log-odds of that average for logistic - which is
    // the same statement written in the units each objective works in.
    let base_score = match config.objective {
        ML_Objective::Squared => mean,
        ML_Objective::Logistic => {
            let clamped = mean.max(1e-6).min(1.0 - 1e-6);
            (clamped / (1.0 - clamped)).ln()
        }
    };

    let mut model = ML_Boost {
        base_score,
        roots: Vec::new(),
        feature: Vec::new(),
        threshold: Vec::new(),
        left: Vec::new(),
        right: Vec::new(),
        default_left: Vec::new(),
        value: Vec::new(),
        gain: Vec::new(),
        columns: columns as i64,
        objective: config.objective,
        trees_used: 0,
    };

    let bins = feature_bins(&features, columns, config.bins as usize);
    let mut running: Vec<f64> = targets.iter().map(|_| base_score).collect();

    // Only used when there is a held-out set to watch.
    let mut validation_running: Vec<f64> = match validation.as_ref() {
        Some((validation_features, _)) => validation_features.iter().map(|_| base_score).collect(),
        None => Vec::new(),
    };
    let mut best_loss = f64::INFINITY;
    let mut best_trees = 0i64;
    let mut rounds_without_improvement = 0i64;

    for _ in 0..config.trees {
        let mut gradients = Vec::with_capacity(targets.len());
        let mut hessians = Vec::with_capacity(targets.len());
        for index in 0..targets.len() {
            let (gradient, hessian) = loss_derivatives(config.objective, running[index], targets[index]);
            gradients.push(gradient);
            hessians.push(hessian);
        }

        let indices: Vec<usize> = (0..features.len()).collect();
        let root = boost_grow(&mut model, &features, &gradients, &hessians, &bins, indices, 0, config);
        model.roots.push(root as i64);
        model.trees_used = model.roots.len() as i64;

        for (index, row) in features.iter().enumerate() {
            running[index] += boost_tree_value(&model, root, row);
        }

        let (validation_features, validation_targets) = match validation.as_ref() {
            Some(held_out) => held_out,
            None => continue,
        };

        for (index, row) in validation_features.iter().enumerate() {
            validation_running[index] += boost_tree_value(&model, root, row);
        }

        let loss = held_out_loss(config.objective, &validation_running, validation_targets);
        if loss < best_loss - 1e-12 {
            best_loss = loss;
            best_trees = model.roots.len() as i64;
            rounds_without_improvement = 0;
        } else {
            rounds_without_improvement += 1;
            if rounds_without_improvement >= config.early_stopping_rounds {
                break;
            }
        }
    }

    // Trees grown after the best one were making the held-out score worse, so
    // they are dropped rather than kept. The nodes they own stay in the arrays
    // and are simply no longer pointed at by any root, which costs a little
    // memory and keeps every other index valid.
    if validation.is_some() && best_trees > 0 {
        model.roots.truncate(best_trees as usize);
        model.trees_used = best_trees;
    }

    return Ok(model);
}

/// The average loss over a held-out set, in whatever units the objective
/// works in - squared error for a number, log loss for a probability.
fn held_out_loss(objective: ML_Objective, raw: &Vec<f64>, targets: &Vec<f64>) -> f64 {
    let mut total = 0.0;
    for index in 0..targets.len() {
        total += match objective {
            ML_Objective::Squared => (raw[index] - targets[index]).powi(2),
            ML_Objective::Logistic => {
                let probability = sigmoid(raw[index]).max(1e-12).min(1.0 - 1e-12);
                -(targets[index] * probability.ln() + (1.0 - targets[index]) * (1.0 - probability).ln())
            }
        };
    }
    return total / targets.len() as f64;
}

/// What the boosted model says about one row: the starting average plus every
/// tree's correction.
pub fn boost_predict(model: &ML_Boost, row: Vec<f64>) -> Result<f64, String> {
    if model.roots.is_empty() {
        return Err("ml_boost_predict: this model has no trees in it".to_string());
    }
    if row.len() as i64 != model.columns {
        return Err(format!("ml_boost_predict: the model was fitted on {} columns but this row has {}", model.columns, row.len()));
    }

    let mut prediction = model.base_score;
    for root in model.roots.iter() {
        prediction += boost_tree_value(model, *root as usize, &row);
    }
    return Ok(prediction);
}

/// What a model fitted with the logistic objective says, as a probability from
/// 0.0 to 1.0 rather than as raw log-odds. Refuses a model fitted to predict a
/// number, because squashing a price through a sigmoid means nothing.
pub fn boost_predict_probability(model: &ML_Boost, row: Vec<f64>) -> Result<f64, String> {
    if model.objective != ML_Objective::Logistic {
        return Err("ml_boost_predict_probability: this model was fitted to predict a number, not a yes-or-no answer - use ml_boost_predict, or fit with ML_Objective::Logistic".to_string());
    }
    return Ok(sigmoid(boost_predict(model, row)?));
}

/// How much each column contributed, as a share of the total: the gain of
/// every split made on that column, added up across every tree, divided by the
/// gain of all splits.
///
/// This is the question worth asking of a model you cannot read. A column near
/// zero is one the model ignored, and dropping it costs nothing but makes
/// everything faster; a column near the top is what the answer actually
/// depends on. Columns come back in their original order, so the answer lines
/// up with the feature names.
pub fn boost_importance(model: &ML_Boost) -> Result<Vec<f64>, String> {
    if model.roots.is_empty() {
        return Err("ml_boost_importance: this model has no trees in it".to_string());
    }

    // Only nodes belonging to trees the model kept are counted. Early
    // stopping leaves the nodes of discarded trees in the arrays, and
    // crediting their splits would report importance for a tree that no
    // longer votes.
    let mut totals = vec![0.0; model.columns as usize];
    let mut pending: Vec<usize> = model.roots.iter().map(|root| *root as usize).collect();
    while let Some(node) = pending.pop() {
        if model.feature[node] < 0 {
            continue;
        }
        totals[model.feature[node] as usize] += model.gain[node];
        pending.push(model.left[node] as usize);
        pending.push(model.right[node] as usize);
    }

    let overall: f64 = totals.iter().sum();
    if overall == 0.0 {
        // No split was ever worth making, so no column is more to blame than
        // any other.
        return Ok(totals);
    }
    return Ok(totals.iter().map(|total| total / overall).collect());
}

/// Judges predicted numbers against real ones, several ways at once.
pub fn regression_scores(predicted: Vec<f64>, actual: Vec<f64>) -> Result<ML_Regression, String> {
    if predicted.len() != actual.len() {
        return Err(format!("ml_regression_scores: {} predictions against {} real values", predicted.len(), actual.len()));
    }
    if predicted.is_empty() {
        return Err("ml_regression_scores: there are no predictions to score".to_string());
    }

    let count = predicted.len() as f64;
    let mean: f64 = actual.iter().sum::<f64>() / count;

    let mut total_squared_error = 0.0;
    let mut total_absolute_error = 0.0;
    let mut total_variation = 0.0;
    let mut percentage_errors: Vec<f64> = Vec::new();
    let mut close_enough = 0.0;

    for index in 0..predicted.len() {
        let error = predicted[index] - actual[index];
        total_squared_error += error * error;
        total_absolute_error += error.abs();
        total_variation += (actual[index] - mean).powi(2);

        // A percentage of zero is not a number, so rows whose real value is
        // zero are left out of the percentage measures rather than making them
        // infinite.
        if actual[index] != 0.0 {
            let share = (error / actual[index]).abs();
            percentage_errors.push(share);
            if share <= 0.1 {
                close_enough += 1.0;
            }
        }
    }

    // Every real value being the same leaves nothing to explain, so there is
    // no share of it a model could explain.
    let r_squared = if total_variation == 0.0 { 0.0 } else { 1.0 - total_squared_error / total_variation };

    let (mape, median_ape, within_ten_percent) = if percentage_errors.is_empty() {
        (0.0, 0.0, 0.0)
    } else {
        let mean_percentage = percentage_errors.iter().sum::<f64>() / percentage_errors.len() as f64;
        let mut sorted = percentage_errors.clone();
        sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        let middle = sorted.len() / 2;
        let median = if sorted.len() % 2 == 0 { (sorted[middle - 1] + sorted[middle]) / 2.0 } else { sorted[middle] };
        (mean_percentage, median, close_enough / percentage_errors.len() as f64)
    };

    return Ok(ML_Regression {
        r_squared,
        mae: total_absolute_error / count,
        rmse: (total_squared_error / count).sqrt(),
        mape,
        median_ape,
        within_ten_percent,
    });
}

#[cfg(test)]
mod boost_tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 1e-6;
    }

    /// A rule a boosted model should learn easily: the answer is mostly the
    /// first column, with a second column that says nothing at all.
    fn priced() -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut features = Vec::new();
        let mut targets = Vec::new();
        for index in 0..80 {
            let size = index as f64;
            let noise = (index % 5) as f64;
            features.push(vec![size, noise]);
            targets.push(3.0 * size + 10.0);
        }
        return (features, targets);
    }

    #[test]
    fn the_default_configuration_is_usable_as_it_stands() {
        let config = boost_default_config();
        assert_eq!(config.trees, 100);
        assert!(config.learning_rate > 0.0 && config.learning_rate < 1.0);
        assert!(config.max_depth >= 1);
        assert!(config.bins >= 2);
    }

    #[test]
    fn a_boosted_model_learns_a_relationship_the_single_tree_only_approximates() {
        let (features, targets) = priced();
        let mut config = boost_default_config();
        config.trees = 60;
        config.min_samples_leaf = 2;
        config.max_depth = 4;

        let model = boost_fit(features.clone(), targets.clone(), &config).expect("a valid configuration");
        let predicted: Vec<f64> = features.iter().map(|row| boost_predict(&model, row.clone()).expect("the right width")).collect();
        let scores = regression_scores(predicted, targets).expect("matching lengths");

        assert!(scores.r_squared > 0.99, "a straight-line rule should be learned almost exactly, got {}", scores.r_squared);
    }

    #[test]
    fn every_tree_makes_the_answer_a_little_better() {
        let (features, targets) = priced();
        let mut config = boost_default_config();
        config.min_samples_leaf = 2;
        config.max_depth = 3;

        let mut previous = f64::INFINITY;
        for trees in [1, 5, 20, 60] {
            config.trees = trees;
            let model = boost_fit(features.clone(), targets.clone(), &config).expect("a valid configuration");
            let predicted: Vec<f64> = features.iter().map(|row| boost_predict(&model, row.clone()).expect("the right width")).collect();
            let scores = regression_scores(predicted, targets.clone()).expect("matching lengths");
            assert!(scores.rmse < previous, "{} trees did not improve on fewer: {} against {}", trees, scores.rmse, previous);
            previous = scores.rmse;
        }
    }

    #[test]
    fn one_tree_alone_only_moves_the_answer_part_of_the_way() {
        // The learning rate is what makes this true, and what makes boosting
        // generalise rather than memorise.
        let (features, targets) = priced();
        let mut config = boost_default_config();
        config.trees = 1;
        config.learning_rate = 0.1;
        config.min_samples_leaf = 2;

        let model = boost_fit(features.clone(), targets.clone(), &config).expect("a valid configuration");
        let predicted = boost_predict(&model, features[79].clone()).expect("the right width");
        let truth = targets[79];
        assert!(predicted > model.base_score, "it moved towards the answer");
        assert!(predicted < truth, "but nowhere near all the way, got {} against {}", predicted, truth);
    }

    #[test]
    fn importance_finds_the_column_that_matters_and_dismisses_the_one_that_does_not() {
        let (features, targets) = priced();
        let mut config = boost_default_config();
        config.trees = 40;
        config.min_samples_leaf = 2;
        config.max_depth = 3;

        let model = boost_fit(features, targets, &config).expect("a valid configuration");
        let importance = boost_importance(&model).expect("a fitted model");

        assert_eq!(importance.len(), 2);
        assert!(importance[0] > 0.9, "the column the answer depends on should carry the gain, got {:?}", importance);
        assert!(importance[1] < 0.1, "the column that says nothing should carry almost none, got {:?}", importance);
        assert!(close(importance.iter().sum::<f64>(), 1.0), "importances are shares of the whole, got {:?}", importance);
    }

    #[test]
    fn a_model_is_judged_on_data_it_never_saw() {
        let (features, targets) = priced();
        let labels: Vec<i64> = targets.iter().map(|value| *value as i64).collect();
        let split = split_train_test(features, labels, 0.75, 99).expect("a valid share");

        let train_targets: Vec<f64> = split.train_labels.iter().map(|label| *label as f64).collect();
        let test_targets: Vec<f64> = split.test_labels.iter().map(|label| *label as f64).collect();

        let mut config = boost_default_config();
        config.trees = 60;
        config.min_samples_leaf = 2;
        config.max_depth = 4;

        let model = boost_fit(split.train_features, train_targets, &config).expect("a valid configuration");
        let predicted: Vec<f64> = split.test_features.iter().map(|row| boost_predict(&model, row.clone()).expect("the right width")).collect();
        let scores = regression_scores(predicted, test_targets).expect("matching lengths");

        assert!(scores.r_squared > 0.95, "held-out accuracy should be high for a learnable rule, got {}", scores.r_squared);

        // The percentage measures are harsher than the absolute ones here, and
        // rightly so: the targets run from 10 to 247, so the same small
        // absolute miss is a large share of the smallest of them. That gap
        // between rmse and within_ten_percent is exactly why both are
        // reported rather than one standing in for the other.
        assert!(scores.within_ten_percent > 0.8, "most predictions should land within a tenth, got {}", scores.within_ten_percent);
        assert!(scores.median_ape < scores.mape, "a few bad small-value rows drag the mean above the median, got {} against {}", scores.mape, scores.median_ape);
    }

    #[test]
    fn a_configuration_that_could_not_work_is_refused() {
        let (features, targets) = priced();
        let mut config = boost_default_config();

        config.trees = 0;
        assert!(boost_fit(features.clone(), targets.clone(), &config).unwrap_err().contains("not a model"));

        config = boost_default_config();
        config.learning_rate = 0.0;
        assert!(boost_fit(features.clone(), targets.clone(), &config).unwrap_err().contains("learning rate"));

        config = boost_default_config();
        config.max_depth = 0;
        assert!(boost_fit(features.clone(), targets.clone(), &config).unwrap_err().contains("no tree to grow"));

        config = boost_default_config();
        config.bins = 1;
        assert!(boost_fit(features.clone(), targets.clone(), &config).unwrap_err().contains("cannot describe a column"));

        config = boost_default_config();
        config.lambda_l2 = -1.0;
        assert!(boost_fit(features, targets, &config).unwrap_err().contains("reward complexity"));
    }

    #[test]
    fn predicting_with_a_row_of_the_wrong_width_is_an_error() {
        let (features, targets) = priced();
        let mut config = boost_default_config();
        config.trees = 2;
        config.min_samples_leaf = 2;
        let model = boost_fit(features, targets, &config).expect("a valid configuration");
        assert!(boost_predict(&model, vec![1.0]).unwrap_err().contains("fitted on 2 columns but this row has 1"));
    }

    #[test]
    fn regression_scores_say_different_things_about_the_same_misses() {
        // Two predictions out by 1, against very different real values: the
        // absolute measures agree, the percentage ones do not.
        let scores = regression_scores(vec![2.0, 101.0], vec![1.0, 100.0]).expect("matching lengths");
        assert!(close(scores.mae, 1.0));
        assert!(close(scores.rmse, 1.0));
        assert!(close(scores.mape, (1.0 + 0.01) / 2.0), "got {}", scores.mape);
        assert!(close(scores.within_ten_percent, 0.5), "one of the two is within a tenth, got {}", scores.within_ten_percent);
    }

    #[test]
    fn a_perfect_prediction_scores_perfectly() {
        let actual = vec![1.0, 2.0, 3.0, 4.0];
        let scores = regression_scores(actual.clone(), actual).expect("matching lengths");
        assert!(close(scores.r_squared, 1.0));
        assert!(close(scores.mae, 0.0));
        assert!(close(scores.rmse, 0.0));
        assert!(close(scores.mape, 0.0));
        assert!(close(scores.within_ten_percent, 1.0));
    }

    #[test]
    fn a_real_value_of_zero_does_not_make_the_percentages_infinite() {
        let scores = regression_scores(vec![1.0, 3.0], vec![0.0, 3.0]).expect("matching lengths");
        assert!(scores.mape.is_finite(), "got {}", scores.mape);
        assert!(close(scores.mape, 0.0), "only the row with a real value counts, got {}", scores.mape);
        assert!(close(scores.mae, 0.5));
    }

    #[test]
    fn scoring_refuses_what_it_cannot_compare() {
        assert!(regression_scores(vec![1.0], vec![1.0, 2.0]).unwrap_err().contains("1 predictions against 2 real values"));
        assert!(regression_scores(vec![], vec![]).unwrap_err().contains("no predictions"));
    }
}

/// A categorical column turned into numbers a model can use, plus the
/// vocabulary that did it - which must be kept, because new data has to be
/// encoded exactly the same way or the columns mean different things.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ML_OneHot {
    pub categories: Vec<String>,
    pub columns: Vec<Vec<f64>>,
}

/// A forest of trees that vote. Held flat like `ML_Boost`, with `roots`
/// pointing at each tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ML_Forest {
    pub roots: Vec<i64>,
    pub feature: Vec<i64>,
    pub threshold: Vec<f64>,
    pub left: Vec<i64>,
    pub right: Vec<i64>,
    pub prediction: Vec<i64>,
    pub columns: i64,
}

/// Turns a column of words into one column of 0s and 1s per distinct word.
///
/// The plain way to give a model a categorical column, and the right one when
/// there are few enough categories - a dozen, not a thousand. The categories
/// come back sorted, so the same input always produces the same columns in the
/// same order, and they must be kept: encoding new data with a different
/// vocabulary silently shifts every column along and produces a model reading
/// the wrong feature.
pub fn one_hot(values: Vec<String>) -> Result<ML_OneHot, String> {
    if values.is_empty() {
        return Err("ml_one_hot: the column is empty, so there is nothing to encode".to_string());
    }

    let mut categories: Vec<String> = values.clone();
    categories.sort();
    categories.dedup();

    let mut columns = Vec::with_capacity(values.len());
    for value in values.iter() {
        let mut row = vec![0.0; categories.len()];
        // The value came from this same column, so it is always found.
        if let Some(at) = categories.iter().position(|category| category == value) {
            row[at] = 1.0;
        }
        columns.push(row);
    }
    return Ok(ML_OneHot { categories, columns });
}

/// Encodes a column of words against a vocabulary already decided - the way to
/// encode new data so it lines up with what a model was trained on. A word
/// that was not in the training data becomes a row of all zeros, which is the
/// honest encoding of "none of the categories I know".
pub fn one_hot_with(values: Vec<String>, categories: Vec<String>) -> Result<Vec<Vec<f64>>, String> {
    if categories.is_empty() {
        return Err("ml_one_hot_with: the vocabulary is empty, so nothing can be encoded against it".to_string());
    }

    let mut columns = Vec::with_capacity(values.len());
    for value in values.iter() {
        let mut row = vec![0.0; categories.len()];
        if let Some(at) = categories.iter().position(|category| category == value) {
            row[at] = 1.0;
        }
        columns.push(row);
    }
    return Ok(columns);
}

/// Replaces each category with the average target for that category, pulled
/// towards the overall average according to how few rows the category has.
///
/// This is what to reach for when one-hot encoding would add a thousand
/// columns - postcodes, product ids, streets. The pulling is not optional
/// decoration: a category with one row would otherwise be encoded as that
/// row's own answer, which hands the model the answer it is supposed to
/// predict and produces a model that scores brilliantly in training and fails
/// completely in use. `smoothing` is how many rows a category needs before it
/// is trusted over the overall average; 10 to 20 is usual.
///
/// Fit this on the training rows only, and apply it to everything else with
/// `ml_encode_with`.
pub fn target_encode(values: Vec<String>, targets: Vec<f64>, smoothing: f64) -> Result<dashmap::DashMap<String, f64>, String> {
    if values.len() != targets.len() {
        return Err(format!("ml_target_encode: {} categories against {} targets", values.len(), targets.len()));
    }
    if values.is_empty() {
        return Err("ml_target_encode: the column is empty, so there is nothing to encode".to_string());
    }
    if smoothing < 0.0 {
        return Err(format!("ml_target_encode: a smoothing of {} is not a number of rows", smoothing));
    }

    let overall: f64 = targets.iter().sum::<f64>() / targets.len() as f64;

    let mut totals: Vec<(String, f64, f64)> = Vec::new();
    for index in 0..values.len() {
        match totals.iter_mut().find(|(category, _, _)| *category == values[index]) {
            Some((_, sum, count)) => {
                *sum += targets[index];
                *count += 1.0;
            }
            None => totals.push((values[index].clone(), targets[index], 1.0)),
        }
    }

    let encoding = dashmap::DashMap::new();
    for (category, sum, count) in totals.iter() {
        let encoded = (sum + overall * smoothing) / (count + smoothing);
        encoding.insert(category.clone(), encoded);
    }
    return Ok(encoding);
}

/// Applies an encoding built by `ml_target_encode` to a column. A category the
/// encoding has never seen becomes the fallback, which should be the overall
/// average of the training targets.
pub fn encode_with(values: Vec<String>, encoding: &dashmap::DashMap<String, f64>, fallback: f64) -> Vec<f64> {
    return values.iter().map(|value| encoding.get(value).map(|found| *found.value()).unwrap_or(fallback)).collect();
}

/// Trains and scores a boosted model `folds` times, each time holding out a
/// different slice, and returns the average of the held-out scores.
///
/// One split on a small dataset says as much about which rows happened to land
/// in the test half as about the model. Every row takes a turn being held out
/// here, so the answer is far steadier - and if the fold scores disagree
/// wildly, that itself is the finding: the model is not learning something
/// stable.
pub fn cross_validate_boost(features: Vec<Vec<f64>>, targets: Vec<f64>, folds: i64, config: &ML_BoostConfig, seed: i64) -> Result<ML_Regression, String> {
    check_dataset("ml_cross_validate_boost", &features, targets.len())?;
    if folds < 2 {
        return Err(format!("ml_cross_validate_boost: {} folds cannot hold anything out", folds));
    }
    if folds as usize > features.len() {
        return Err(format!("ml_cross_validate_boost: asked for {} folds from {} rows", folds, features.len()));
    }

    let mut order: Vec<usize> = (0..features.len()).collect();
    let mut shuffler = Shuffler::new(seed);
    let mut position = order.len();
    while position > 1 {
        position -= 1;
        let swap_with = shuffler.below(position + 1);
        order.swap(position, swap_with);
    }

    let mut predicted = vec![0.0; features.len()];
    for fold in 0..folds as usize {
        let mut train_features = Vec::new();
        let mut train_targets = Vec::new();
        let mut held_out = Vec::new();

        for (place, row_index) in order.iter().enumerate() {
            if place % folds as usize == fold {
                held_out.push(*row_index);
            } else {
                train_features.push(features[*row_index].clone());
                train_targets.push(targets[*row_index]);
            }
        }

        if held_out.is_empty() || train_features.is_empty() {
            continue;
        }

        let model = boost_fit(train_features, train_targets, config)?;
        for row_index in held_out.iter() {
            predicted[*row_index] = boost_predict(&model, features[*row_index].clone())?;
        }
    }

    // Every row was predicted by a model that had not seen it, so scoring them
    // all together is scoring held-out predictions throughout.
    return regression_scores(predicted, targets);
}

/// Fits a forest of decision trees, each grown on a different random sample of
/// the rows, and predicts by letting them vote.
///
/// The point is that the errors of the individual trees are different from
/// each other, so voting cancels most of them out. That makes a forest far
/// harder to get badly wrong than a single tree and far less sensitive to
/// settings than boosting - which is exactly when to reach for it: when there
/// is no time to tune anything.
///
/// Each tree sees a sample of the rows drawn with replacement, of the same size
/// as the original - the bootstrap, which is what makes the trees differ.
pub fn forest_fit(features: Vec<Vec<f64>>, labels: Vec<i64>, trees: i64, max_depth: i64, seed: i64) -> Result<ML_Forest, String> {
    let columns = check_dataset("ml_forest_fit", &features, labels.len())?;
    if trees < 1 {
        return Err(format!("ml_forest_fit: {} trees is not a forest", trees));
    }
    if max_depth < 1 {
        return Err(format!("ml_forest_fit: a maximum depth of {} leaves no tree to grow", max_depth));
    }

    let mut forest = ML_Forest { roots: Vec::new(), feature: Vec::new(), threshold: Vec::new(), left: Vec::new(), right: Vec::new(), prediction: Vec::new(), columns: columns as i64 };
    let mut shuffler = Shuffler::new(seed);

    for _ in 0..trees {
        let mut sample_features = Vec::with_capacity(features.len());
        let mut sample_labels = Vec::with_capacity(features.len());
        for _ in 0..features.len() {
            let drawn = shuffler.below(features.len());
            sample_features.push(features[drawn].clone());
            sample_labels.push(labels[drawn]);
        }

        let tree = tree_fit(sample_features, sample_labels, max_depth)?;

        // The tree's node indices are relative to itself, so they are shifted
        // as it is appended to the forest's flat arrays.
        let offset = forest.feature.len() as i64;
        forest.roots.push(offset);
        for node in 0..tree.feature.len() {
            forest.feature.push(tree.feature[node]);
            forest.threshold.push(tree.threshold[node]);
            forest.left.push(if tree.left[node] < 0 { -1 } else { tree.left[node] + offset });
            forest.right.push(if tree.right[node] < 0 { -1 } else { tree.right[node] + offset });
            forest.prediction.push(tree.prediction[node]);
        }
    }

    return Ok(forest);
}

/// What the forest says about one row: the answer most of its trees give.
pub fn forest_predict(forest: &ML_Forest, row: Vec<f64>) -> Result<i64, String> {
    if forest.roots.is_empty() {
        return Err("ml_forest_predict: this forest has no trees in it".to_string());
    }
    if row.len() as i64 != forest.columns {
        return Err(format!("ml_forest_predict: the forest was fitted on {} columns but this row has {}", forest.columns, row.len()));
    }

    let mut votes = Vec::with_capacity(forest.roots.len());
    for root in forest.roots.iter() {
        let mut at = *root as usize;
        for _ in 0..forest.feature.len() + 1 {
            if forest.feature[at] < 0 {
                votes.push(forest.prediction[at]);
                break;
            }
            at = if row[forest.feature[at] as usize] <= forest.threshold[at] { forest.left[at] as usize } else { forest.right[at] as usize };
        }
    }
    return Ok(majority(&votes));
}

#[cfg(test)]
mod extra_tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 1e-6;
    }

    /// Rows where the answer is a yes-or-no, learnable from the first column.
    fn yes_or_no() -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut features = Vec::new();
        let mut targets = Vec::new();
        for index in 0..60 {
            let value = index as f64;
            features.push(vec![value, (index % 4) as f64]);
            targets.push(if value > 30.0 { 1.0 } else { 0.0 });
        }
        return (features, targets);
    }

    #[test]
    fn the_logistic_objective_predicts_probabilities() {
        let (features, targets) = yes_or_no();
        let mut config = boost_default_config();
        config.objective = ML_Objective::Logistic;
        config.trees = 40;
        config.min_samples_leaf = 2;
        config.max_depth = 3;

        let model = boost_fit(features, targets, &config).expect("a valid configuration");

        let low = boost_predict_probability(&model, vec![5.0, 1.0]).expect("a logistic model");
        let high = boost_predict_probability(&model, vec![55.0, 1.0]).expect("a logistic model");
        assert!((0.0..=1.0).contains(&low), "a probability, got {}", low);
        assert!((0.0..=1.0).contains(&high), "a probability, got {}", high);
        assert!(low < 0.2, "a clearly negative row should be near zero, got {}", low);
        assert!(high > 0.8, "a clearly positive row should be near one, got {}", high);
    }

    #[test]
    fn the_logistic_objective_insists_on_yes_or_no_targets() {
        let mut config = boost_default_config();
        config.objective = ML_Objective::Logistic;
        let features = vec![vec![1.0], vec![2.0], vec![3.0]];
        assert!(boost_fit(features, vec![0.0, 1.0, 7.5], &config).unwrap_err().contains("every target to be 0 or 1"));
    }

    #[test]
    fn probabilities_are_refused_for_a_model_that_predicts_a_number() {
        let (features, _) = yes_or_no();
        let targets: Vec<f64> = features.iter().map(|row| row[0] * 2.0).collect();
        let mut config = boost_default_config();
        config.trees = 5;
        config.min_samples_leaf = 2;

        let model = boost_fit(features, targets, &config).expect("a valid configuration");
        assert!(boost_predict_probability(&model, vec![1.0, 1.0]).unwrap_err().contains("fitted to predict a number"));
    }

    #[test]
    fn early_stopping_keeps_fewer_trees_than_it_was_offered() {
        // A rule this simple is learned in a handful of trees; the rest only
        // make the held-out score worse.
        let (features, targets) = yes_or_no();
        let split_labels: Vec<i64> = targets.iter().map(|target| *target as i64).collect();
        let split = split_train_test(features, split_labels, 0.7, 11).expect("a valid share");
        let train_targets: Vec<f64> = split.train_labels.iter().map(|label| *label as f64).collect();
        let validation_targets: Vec<f64> = split.test_labels.iter().map(|label| *label as f64).collect();

        let mut config = boost_default_config();
        config.trees = 500;
        config.early_stopping_rounds = 5;
        config.min_samples_leaf = 2;
        config.max_depth = 3;

        let model = boost_fit_validated(split.train_features, train_targets, split.test_features, validation_targets, &config).expect("a valid configuration");

        assert!(model.trees_used < 500, "training should have stopped early, used {}", model.trees_used);
        assert!(model.trees_used >= 1, "it should have kept at least one tree");
        assert_eq!(model.roots.len() as i64, model.trees_used, "the kept trees and the count must agree");
    }

    #[test]
    fn early_stopping_needs_a_rule_to_stop_by() {
        let (features, targets) = yes_or_no();
        let mut config = boost_default_config();
        config.early_stopping_rounds = 0;
        let failure = boost_fit_validated(features.clone(), targets.clone(), features, targets, &config).unwrap_err();
        assert!(failure.contains("is not a rule"), "got: {}", failure);
    }

    #[test]
    fn validation_rows_must_be_the_same_shape_as_the_training_rows() {
        let (features, targets) = yes_or_no();
        let config = boost_default_config();
        let failure = boost_fit_validated(features, targets, vec![vec![1.0]], vec![1.0], &config).unwrap_err();
        assert!(failure.contains("training rows have 2 columns but the validation rows have 1"), "got: {}", failure);
    }

    #[test]
    fn importance_counts_only_the_trees_that_were_kept() {
        let (features, targets) = yes_or_no();
        let split_labels: Vec<i64> = targets.iter().map(|target| *target as i64).collect();
        let split = split_train_test(features, split_labels, 0.7, 3).expect("a valid share");
        let train_targets: Vec<f64> = split.train_labels.iter().map(|label| *label as f64).collect();
        let validation_targets: Vec<f64> = split.test_labels.iter().map(|label| *label as f64).collect();

        let mut config = boost_default_config();
        config.trees = 200;
        config.early_stopping_rounds = 3;
        config.min_samples_leaf = 2;
        config.max_depth = 3;

        let model = boost_fit_validated(split.train_features, train_targets, split.test_features, validation_targets, &config).expect("a valid configuration");
        let importance = boost_importance(&model).expect("a fitted model");
        assert!(close(importance.iter().sum::<f64>(), 1.0), "importances are shares of the whole, got {:?}", importance);
        assert!(importance[0] > importance[1], "the column that matters should still lead, got {:?}", importance);
    }

    #[test]
    fn a_missing_value_is_learned_rather_than_guessed_at() {
        // Rows whose first column is absent are all positive, and nothing else
        // says so. A model that ignored missingness could not do better than
        // chance on them; one that routes them deliberately gets them right.
        let mut features = Vec::new();
        let mut targets = Vec::new();
        for index in 0..40 {
            features.push(vec![index as f64, 1.0]);
            targets.push(if index > 20 { 1.0 } else { 0.0 });
        }
        for _ in 0..20 {
            features.push(vec![f64::NAN, 1.0]);
            targets.push(1.0);
        }

        let mut config = boost_default_config();
        config.objective = ML_Objective::Logistic;
        config.trees = 40;
        config.min_samples_leaf = 2;
        config.max_depth = 3;

        let model = boost_fit(features, targets, &config).expect("a valid configuration");
        let missing = boost_predict_probability(&model, vec![f64::NAN, 1.0]).expect("a logistic model");
        assert!(missing > 0.8, "a row with the column absent should be read as positive, got {}", missing);

        let low = boost_predict_probability(&model, vec![2.0, 1.0]).expect("a logistic model");
        assert!(low < 0.3, "a small present value should still be negative, got {}", low);
    }

    #[test]
    fn a_single_tree_refuses_gaps_rather_than_deciding_for_you() {
        let features = vec![vec![1.0, 2.0], vec![f64::NAN, 3.0], vec![4.0, 5.0]];
        let failure = tree_fit(features, vec![0, 1, 1], 3).unwrap_err();
        assert!(failure.contains("row 1 has a column with no value"), "got: {}", failure);
        assert!(failure.contains("ml_boost_fit"), "the error should point at what does handle gaps: {}", failure);
    }

    #[test]
    fn one_hot_gives_a_column_per_category_in_a_stable_order() {
        let encoded = one_hot(vec!["red".to_string(), "blue".to_string(), "red".to_string()]).expect("a filled column");
        assert_eq!(encoded.categories, vec!["blue".to_string(), "red".to_string()], "categories come back sorted");
        assert_eq!(encoded.columns[0], vec![0.0, 1.0]);
        assert_eq!(encoded.columns[1], vec![1.0, 0.0]);
        assert_eq!(encoded.columns[2], vec![0.0, 1.0]);
    }

    #[test]
    fn one_hot_against_a_known_vocabulary_lines_new_data_up_with_old() {
        let trained = one_hot(vec!["red".to_string(), "blue".to_string(), "green".to_string()]).expect("a filled column");
        let fresh = one_hot_with(vec!["green".to_string(), "purple".to_string()], trained.categories.clone()).expect("a vocabulary");

        assert_eq!(fresh[0], vec![0.0, 1.0, 0.0], "green sits where it did in training");
        assert_eq!(fresh[1], vec![0.0, 0.0, 0.0], "a category never seen is all zeros, not a wrong one");
    }

    #[test]
    fn target_encoding_pulls_thin_categories_towards_the_overall_average() {
        // "common" has plenty of rows and keeps its own average; "rare" has one
        // row and is pulled most of the way back to the overall average.
        let values = vec![
            "common".to_string(),
            "common".to_string(),
            "common".to_string(),
            "common".to_string(),
            "common".to_string(),
            "common".to_string(),
            "common".to_string(),
            "common".to_string(),
            "rare".to_string(),
        ];
        let targets = vec![10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 100.0];
        let overall: f64 = targets.iter().sum::<f64>() / targets.len() as f64;

        // Smoothing of 2 means a category is trusted over the overall average
        // once it has a couple of rows behind it.
        let encoding = target_encode(values, targets, 2.0).expect("matching lengths");
        let rare = *encoding.get("rare").expect("the category").value();
        let common = *encoding.get("common").expect("the category").value();

        assert!(rare < 100.0, "a category of one row must not be encoded as its own answer, got {}", rare);
        assert!((rare - overall).abs() < (100.0 - overall).abs() / 2.0, "it should be pulled most of the way back, got {}", rare);
        assert!((common - 10.0).abs() < 5.0, "a well-populated category keeps its own average, got {}", common);
    }

    #[test]
    fn an_unseen_category_encodes_to_the_fallback() {
        let encoding = target_encode(vec!["a".to_string(), "b".to_string()], vec![1.0, 3.0], 0.0).expect("matching lengths");
        let encoded = encode_with(vec!["a".to_string(), "never seen".to_string()], &encoding, 2.0);
        assert!(close(encoded[0], 1.0));
        assert!(close(encoded[1], 2.0), "the fallback stands in for what the encoding does not know");
    }

    #[test]
    fn target_encoding_refuses_what_it_cannot_do() {
        assert!(target_encode(vec!["a".to_string()], vec![1.0, 2.0], 1.0).unwrap_err().contains("1 categories against 2 targets"));
        assert!(target_encode(vec![], vec![], 1.0).unwrap_err().contains("empty"));
        assert!(target_encode(vec!["a".to_string()], vec![1.0], -1.0).unwrap_err().contains("not a number of rows"));
        assert!(one_hot(vec![]).unwrap_err().contains("empty"));
    }

    #[test]
    fn cross_validation_scores_every_row_on_a_model_that_did_not_see_it() {
        let mut features = Vec::new();
        let mut targets = Vec::new();
        for index in 0..50 {
            let value = index as f64;
            features.push(vec![value, (index % 3) as f64]);
            targets.push(2.0 * value + 5.0);
        }

        let mut config = boost_default_config();
        config.trees = 40;
        config.min_samples_leaf = 2;
        config.max_depth = 4;

        let scores = cross_validate_boost(features, targets, 5, &config, 1234).expect("a valid fold count");
        assert!(scores.r_squared > 0.9, "a learnable rule should survive cross-validation, got {}", scores.r_squared);
    }

    #[test]
    fn cross_validation_refuses_fold_counts_that_hold_nothing_out() {
        let features = vec![vec![1.0], vec![2.0], vec![3.0]];
        let targets = vec![1.0, 2.0, 3.0];
        let config = boost_default_config();
        assert!(cross_validate_boost(features.clone(), targets.clone(), 1, &config, 1).unwrap_err().contains("cannot hold anything out"));
        assert!(cross_validate_boost(features, targets, 9, &config, 1).unwrap_err().contains("9 folds from 3 rows"));
    }

    #[test]
    fn a_forest_votes_and_gets_the_rule_right() {
        let mut features = Vec::new();
        let mut labels = Vec::new();
        for index in 0..40 {
            let value = index as f64;
            features.push(vec![value, (index % 5) as f64]);
            labels.push(if value > 20.0 { 1 } else { 0 });
        }

        let forest = forest_fit(features, labels, 15, 4, 7).expect("a valid forest");
        assert_eq!(forest.roots.len(), 15);
        assert_eq!(forest_predict(&forest, vec![2.0, 1.0]).expect("the right width"), 0);
        assert_eq!(forest_predict(&forest, vec![38.0, 1.0]).expect("the right width"), 1);
    }

    #[test]
    fn a_forest_answers_the_same_way_for_the_same_seed() {
        let features = vec![vec![1.0], vec![2.0], vec![8.0], vec![9.0], vec![10.0], vec![11.0]];
        let labels = vec![0, 0, 1, 1, 1, 1];
        let first = forest_fit(features.clone(), labels.clone(), 8, 3, 5).expect("a valid forest");
        let second = forest_fit(features, labels, 8, 3, 5).expect("a valid forest");
        assert_eq!(first.feature, second.feature);
        assert_eq!(first.threshold, second.threshold);
    }

    #[test]
    fn a_forest_refuses_what_it_cannot_grow() {
        let features = vec![vec![1.0], vec![2.0]];
        let labels = vec![0, 1];
        assert!(forest_fit(features.clone(), labels.clone(), 0, 3, 1).unwrap_err().contains("not a forest"));
        assert!(forest_fit(features, labels, 3, 0, 1).unwrap_err().contains("no tree to grow"));

        let empty = ML_Forest { roots: vec![], feature: vec![], threshold: vec![], left: vec![], right: vec![], prediction: vec![], columns: 1 };
        assert!(forest_predict(&empty, vec![1.0]).unwrap_err().contains("no trees"));
    }
}
