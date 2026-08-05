//! The standard library, described as data.
//!
//! Every entry here is read from the registry the compiler itself type checks
//! against, so a program listing these functions lists exactly what the
//! compiler that built it can call - a hand-kept list would drift the first
//! time a function was added.

use crate::stdlib_registry::{StdlibModule, STDLIB_FUNCTIONS};

/// One standard library function, spelled the way it is written in Nail.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct STDLIB_Function {
    pub name: String,
    /// The namespace the function belongs to, spelled the way a call spells
    /// it (`string`, `db`).
    pub module: String,
    /// The call as a Nail declaration reads it: `string_split(input:s, delimiter:s):a:s`.
    pub signature: String,
    pub description: String,
    pub example: String,
}

/// Every callable standard library function, sorted by module name and then
/// by function name, so a person can find things without knowing the
/// registry's internal order.
pub fn functions() -> Vec<STDLIB_Function> {
    let mut functions: Vec<STDLIB_Function> = STDLIB_FUNCTIONS
        .iter()
        .map(|(name, function)| {
            let parameters: Vec<String> = function.parameters.iter().map(|parameter| format!("{}:{}", parameter.name, parameter.param_type)).collect();
            STDLIB_Function {
                name: name.to_string(),
                module: function.module.display_name().to_string(),
                signature: format!("{}({}):{}", name, parameters.join(", "), function.return_type),
                description: function.description.to_string(),
                example: function.example.to_string(),
            }
        })
        .collect();
    functions.sort_by(|left, right| left.module.cmp(&right.module).then_with(|| left.name.cmp(&right.name)));
    functions
}

/// The namespaces that actually export something, alphabetically - the order
/// `functions` lists them. Modules that share a namespace (SQLite, Postgres
/// and DataFusion are all `db`) appear once.
pub fn modules() -> Vec<String> {
    let exported: Vec<&StdlibModule> = STDLIB_FUNCTIONS.values().map(|function| &function.module).collect();
    let mut names: Vec<String> = Vec::new();
    for module in StdlibModule::all().iter().filter(|module| exported.contains(module)) {
        let name = module.display_name().to_string();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_function_belongs_to_a_listed_module_and_no_module_is_listed_twice() {
        let modules = modules();
        let mut seen = std::collections::HashSet::new();
        for module in &modules {
            assert!(seen.insert(module.clone()), "module {} listed twice - dedup display names that several modules share", module);
        }
        for function in functions() {
            assert!(modules.contains(&function.module), "function {} claims module {} which modules() does not list", function.name, function.module);
        }
    }

    #[test]
    fn functions_that_share_a_module_sit_together_in_the_listing() {
        let listed = functions();
        let mut last_position: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (position, function) in listed.iter().enumerate() {
            if let Some(previous) = last_position.get(&function.module) {
                assert_eq!(position, previous + 1, "module {} is split - {} appears after functions from another module", function.module, function.name);
            }
            last_position.insert(function.module.clone(), position);
        }
    }
}
