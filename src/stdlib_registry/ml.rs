//! Machine learning module stdlib registry entries.
//!
//! A fitted model is a plain struct, not a handle to something living inside
//! the library, so it can be printed, stored as JSON and predicted with later.
//! Everything involving randomness takes the seed as an argument.

use super::*;

/// A fitted model handed back in, by reference.
fn model_parameter(name: &'static str, type_name: &'static str) -> StdlibParameter {
    return StdlibParameter { name: name.to_string(), param_type: NailDataTypeDescriptor::Struct(type_name.to_string()), pass_by_reference: true };
}

/// A result carrying one of this module's structs.
fn returns(type_name: &'static str) -> NailDataTypeDescriptor {
    return NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct(type_name.to_string())));
}

/// One function that takes a fitted model and gives an answer about a row.
/// The model type has to be imported wherever the function is used, and a
/// bare struct name is not something the short form can express, so these are
/// built here rather than written out five times.
fn about_a_model(rust_path: &'static str, model_type: &'static str, extra: Vec<StdlibParameter>, return_type: NailDataTypeDescriptor, description: &'static str, example: &'static str) -> StdlibFunction {
    let mut parameters = vec![model_parameter("model", model_type)];
    parameters.extend(extra);
    return StdlibFunction {
        rust_path: rust_path.to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        // A model struct that carries an enum needs that enum imported too,
        // the same way CSV_Options brings CSV_Trim with it.
        custom_type_imports: if model_type == "ML_Boost" {
            vec![(model_type, "nail::std_lib::ml"), ("ML_Objective", "nail::std_lib::ml")]
        } else {
            vec![(model_type, "nail::std_lib::ml")]
        },
        module: StdlibModule::Ml,
        parameters,
        return_type,
        diverging: false,
        description,
        example,
    };
}

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Ml:
        "ml_normalize" => "std_lib::ml::normalize", (values: [f]) -> ([f]!e),
            "Rescales values so the smallest becomes 0.0 and the largest 1.0. Do this before any model that measures distance, so a column in millions does not drown out one in single digits.",
            "scaled:a:f = danger(ml_normalize(prices));";
        "ml_standardize" => "std_lib::ml::standardize", (values: [f]) -> ([f]!e),
            "Rescales values to sit around zero with a spread of one. The other way of putting columns on the same footing, and the one to use when outliers matter.",
            "scaled:a:f = danger(ml_standardize(prices));";
        "ml_knn_predict" => "std_lib::ml::knn_predict", (features: [[f]], labels: [i], query: [f], k: i) -> (i!e),
            "Predicts a label by asking the k nearest rows what they are. No fitting happens - the data is the model - so this is what to reach for when there is very little of it.",
            "label:i = danger(ml_knn_predict(rows, labels, query, 3));";
    }

    m.insert(
        "ml_linear_predict",
        about_a_model(
            "std_lib::ml::linear_predict",
            "ML_Linear",
            vec![nail_param!(row: [f])],
            nail_type!((f!e)),
            "What a fitted line says about one row.",
            "guess:f = danger(ml_linear_predict(model, row));",
        ),
    );

    m.insert(
        "ml_tree_predict",
        about_a_model(
            "std_lib::ml::tree_predict",
            "ML_Tree",
            vec![nail_param!(row: [f])],
            nail_type!((i!e)),
            "What a fitted tree says about one row.",
            "label:i = danger(ml_tree_predict(tree, row));",
        ),
    );

    m.insert(
        "ml_tree_explain",
        about_a_model(
            "std_lib::ml::tree_explain",
            "ML_Tree",
            vec![nail_param!(feature_names: [s])],
            nail_type!((s!e)),
            "Writes a tree out as the rules it actually applies - the reason to reach for a tree over something more accurate. Pass an empty array to see the columns numbered.",
            "rules:s = danger(ml_tree_explain(tree, [`size`, `weight`]));",
        ),
    );

    m.insert(
        "ml_boost_predict",
        about_a_model(
            "std_lib::ml::boost_predict",
            "ML_Boost",
            vec![nail_param!(row: [f])],
            nail_type!((f!e)),
            "What a boosted model says about one row: the starting average plus every tree's correction.",
            "estimate:f = danger(ml_boost_predict(model, row));",
        ),
    );

    m.insert(
        "ml_boost_importance",
        about_a_model(
            "std_lib::ml::boost_importance",
            "ML_Boost",
            vec![],
            nail_type!(([f]!e)),
            "How much each column contributed, as a share of the total gain, in the original column order. A column near zero is one the model ignored, and dropping it costs nothing.",
            "shares:a:f = danger(ml_boost_importance(model));",
        ),
    );

    m.insert("ml_split_train_test", StdlibFunction {
        rust_path: "std_lib::ml::split_train_test".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_Split", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(features: [[f]]), nail_param!(labels: [i]), nail_param!(train_share: f), nail_param!(seed: i)],
        return_type: returns("ML_Split"),
        diverging: false,
        description: "Cuts a dataset into a part to learn from and a part to be judged on, shuffling first so an ordering in the file does not become an ordering in the split. The seed makes the cut reproducible.",
        example: "split:ML_Split = danger(ml_split_train_test(rows, labels, 0.8, 42));",
    });

    m.insert("ml_linear_fit", StdlibFunction {
        rust_path: "std_lib::ml::linear_fit".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_Linear", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(features: [[f]]), nail_param!(targets: [f])],
        return_type: returns("ML_Linear"),
        diverging: false,
        description: "Fits the straight line closest to the data, exactly rather than iteratively - no learning rate to tune. Errors when two columns say the same thing, because then no single line fits best.",
        example: "model:ML_Linear = danger(ml_linear_fit(rows, targets));",
    });

    m.insert("ml_tree_fit", StdlibFunction {
        rust_path: "std_lib::ml::tree_fit".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_Tree", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(features: [[f]]), nail_param!(labels: [i]), nail_param!(max_depth: i)],
        return_type: returns("ML_Tree"),
        diverging: false,
        description: "Fits a decision tree by repeatedly splitting on whichever column separates the classes best. The maximum depth is what stands between a useful model and one that has memorised the training set - three to five is a sensible start.",
        example: "tree:ML_Tree = danger(ml_tree_fit(rows, labels, 4));",
    });

    m.insert("ml_kmeans", StdlibFunction {
        rust_path: "std_lib::ml::kmeans".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_Clusters", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(points: [[f]]), nail_param!(k: i), nail_param!(seed: i), nail_param!(iterations: i)],
        return_type: returns("ML_Clusters"),
        diverging: false,
        description: "Groups points by nearness into k groups. The starting points come from the seed and the answer depends on them, which is why the seed is an argument rather than a hidden decision.",
        example: "clusters:ML_Clusters = danger(ml_kmeans(points, 3, 42, 20));",
    });

    m.insert("ml_score", StdlibFunction {
        rust_path: "std_lib::ml::score".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_Scores", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(predicted: [i]), nail_param!(actual: [i])],
        return_type: returns("ML_Scores"),
        diverging: false,
        description: "Counts how a set of predictions did, treating the label 1 as positive. All four numbers come back together because accuracy alone flatters a model that never says yes.",
        example: "scores:ML_Scores = danger(ml_score(predicted, actual));",
    });

    m.insert("ml_boost_default_config", StdlibFunction {
        rust_path: "std_lib::ml::boost_default_config".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_BoostConfig", "nail::std_lib::ml"), ("ML_Objective", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Struct("ML_BoostConfig".to_string()),
        diverging: false,
        description: "Sensible values to start a boosted model from: 100 trees, a learning rate slow enough that no single tree dominates, and a depth shallow enough to generalise.",
        example: "config:ML_BoostConfig = ml_boost_default_config();",
    });

    m.insert("ml_boost_fit", StdlibFunction {
        rust_path: "std_lib::ml::boost_fit".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_Boost", "nail::std_lib::ml"), ("ML_BoostConfig", "nail::std_lib::ml"), ("ML_Objective", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(features: [[f]]), nail_param!(targets: [f]), model_parameter("config", "ML_BoostConfig")],
        return_type: returns("ML_Boost"),
        diverging: false,
        description: "Fits a gradient boosting model - many small trees, each trained on what the ones before it still get wrong. The method that wins on ordinary tabular data, and the one LightGBM and XGBoost implement. Predicts a number; for yes-or-no questions fit against 0 and 1.",
        example: "model:ML_Boost = danger(ml_boost_fit(rows, prices, config));",
    });

    m.insert("ml_regression_scores", StdlibFunction {
        rust_path: "std_lib::ml::regression_scores".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_Regression", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(predicted: [f]), nail_param!(actual: [f])],
        return_type: returns("ML_Regression"),
        diverging: false,
        description: "Judges predicted numbers against real ones several ways at once: r_squared, mae, rmse, mape, median_ape and within_ten_percent. Rows whose real value is zero are left out of the percentage measures rather than making them infinite.",
        example: "scores:ML_Regression = danger(ml_regression_scores(predicted, actual));",
    });

    m.insert(
        "ml_boost_predict_probability",
        about_a_model(
            "std_lib::ml::boost_predict_probability",
            "ML_Boost",
            vec![nail_param!(row: [f])],
            nail_type!((f!e)),
            "What a model fitted with ML_Objective::Logistic says, as a probability from 0.0 to 1.0. Refuses a model fitted to predict a number.",
            "chance:f = danger(ml_boost_predict_probability(model, row));",
        ),
    );

    m.insert(
        "ml_forest_predict",
        about_a_model(
            "std_lib::ml::forest_predict",
            "ML_Forest",
            vec![nail_param!(row: [f])],
            nail_type!((i!e)),
            "What the forest says about one row: the answer most of its trees give.",
            "label:i = danger(ml_forest_predict(forest, row));",
        ),
    );

    m.insert("ml_boost_fit_validated", StdlibFunction {
        rust_path: "std_lib::ml::boost_fit_validated".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_Boost", "nail::std_lib::ml"), ("ML_BoostConfig", "nail::std_lib::ml"), ("ML_Objective", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![
            nail_param!(features: [[f]]),
            nail_param!(targets: [f]),
            nail_param!(validation_features: [[f]]),
            nail_param!(validation_targets: [f]),
            model_parameter("config", "ML_BoostConfig"),
        ],
        return_type: returns("ML_Boost"),
        diverging: false,
        description: "Fits a boosted model while watching a held-out set, and stops once that set stops improving - the answer to the only hard question ml_boost_fit asks, which is how many trees. Trees grown after the best one are thrown away.",
        example: "model:ML_Boost = danger(ml_boost_fit_validated(train_rows, train_prices, test_rows, test_prices, config));",
    });

    m.insert("ml_cross_validate_boost", StdlibFunction {
        rust_path: "std_lib::ml::cross_validate_boost".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_BoostConfig", "nail::std_lib::ml"), ("ML_Objective", "nail::std_lib::ml"), ("ML_Regression", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(features: [[f]]), nail_param!(targets: [f]), nail_param!(folds: i), model_parameter("config", "ML_BoostConfig"), nail_param!(seed: i)],
        return_type: returns("ML_Regression"),
        diverging: false,
        description: "Trains and scores a boosted model once per fold, holding out a different slice each time, and averages the held-out scores. One split on a small dataset says as much about which rows landed where as about the model; this does not.",
        example: "scores:ML_Regression = danger(ml_cross_validate_boost(rows, prices, 5, config, 42));",
    });

    m.insert("ml_forest_fit", StdlibFunction {
        rust_path: "std_lib::ml::forest_fit".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_Forest", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(features: [[f]]), nail_param!(labels: [i]), nail_param!(trees: i), nail_param!(max_depth: i), nail_param!(seed: i)],
        return_type: returns("ML_Forest"),
        diverging: false,
        description: "Fits a forest of trees, each grown on a different random sample of the rows, that predict by voting. Far harder to get badly wrong than a single tree and far less sensitive to settings than boosting - reach for it when there is no time to tune anything.",
        example: "forest:ML_Forest = danger(ml_forest_fit(rows, labels, 50, 6, 42));",
    });

    m.insert("ml_one_hot", StdlibFunction {
        rust_path: "std_lib::ml::one_hot".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ML_OneHot", "nail::std_lib::ml")],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(values: [s])],
        return_type: returns("ML_OneHot"),
        diverging: false,
        description: "Turns a column of words into one column of 0s and 1s per distinct word, with the sorted vocabulary that did it. Keep the vocabulary - new data must be encoded against the same one or every column shifts along.",
        example: "encoded:ML_OneHot = danger(ml_one_hot(colours));",
    });

    m.insert("ml_target_encode", StdlibFunction {
        rust_path: "std_lib::ml::target_encode".to_string(),
        crate_deps: vec![CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(values: [s]), nail_param!(targets: [f]), nail_param!(smoothing: f)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::Float)))),
        diverging: false,
        description: "Replaces each category with its average target, pulled towards the overall average according to how few rows it has. For columns where one-hot would add a thousand columns. Fit on training rows only - the smoothing is what stops a one-row category being encoded as its own answer.",
        example: "encoding:h<s,f> = danger(ml_target_encode(postcodes, prices, 20.0));",
    });

    m.insert("ml_one_hot_with", StdlibFunction {
        rust_path: "std_lib::ml::one_hot_with".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Ml,
        parameters: vec![nail_param!(values: [s]), nail_param!(categories: [s])],
        return_type: nail_type!(([[f]]!e)),
        diverging: false,
        description: "Encodes a column against a vocabulary already decided, so new data lines up with what a model was trained on. A word that was not in the training data becomes all zeros.",
        example: "rows:a:a:f = danger(ml_one_hot_with(colours, encoded.categories));",
    });

    m.insert("ml_encode_with", StdlibFunction {
        rust_path: "std_lib::ml::encode_with".to_string(),
        crate_deps: vec![CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Ml,
        parameters: vec![
            nail_param!(values: [s]),
            StdlibParameter { name: "encoding".to_string(), param_type: NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::Float)), pass_by_reference: true },
            nail_param!(fallback: f),
        ],
        return_type: nail_type!([f]),
        diverging: false,
        description: "Applies an encoding from ml_target_encode to a column. A category the encoding has never seen becomes the fallback, which should be the overall average of the training targets.",
        example: "encoded:a:f = ml_encode_with(postcodes, encoding, average_price);",
    });
}
