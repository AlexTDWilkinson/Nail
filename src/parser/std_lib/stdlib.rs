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
    /// The module the function belongs to, by its display name (`Strings`).
    pub module: String,
    /// The call as a Nail declaration reads it: `string_split(input:s, delimiter:s):a:s`.
    pub signature: String,
    pub description: String,
    pub example: String,
}

/// Every callable standard library function, sorted by module in the order the
/// registry declares them, then by name.
pub fn functions() -> Vec<STDLIB_Function> {
    let module_order = StdlibModule::all();
    let mut functions: Vec<(usize, STDLIB_Function)> = STDLIB_FUNCTIONS
        .iter()
        .map(|(name, function)| {
            let parameters: Vec<String> = function.parameters.iter().map(|parameter| format!("{}:{}", parameter.name, parameter.param_type)).collect();
            let order = module_order.iter().position(|module| module == &function.module).unwrap_or(module_order.len());
            (
                order,
                STDLIB_Function {
                    name: name.to_string(),
                    module: function.module.display_name().to_string(),
                    signature: format!("{}({}):{}", name, parameters.join(", "), function.return_type),
                    description: function.description.to_string(),
                    example: function.example.to_string(),
                },
            )
        })
        .collect();
    functions.sort_by(|(left_order, left), (right_order, right)| left_order.cmp(right_order).then_with(|| left.name.cmp(&right.name)));
    functions.into_iter().map(|(_, function)| function).collect()
}

/// The modules that actually export something, by display name, in the order
/// `functions` lists them.
pub fn modules() -> Vec<String> {
    let exported: Vec<&StdlibModule> = STDLIB_FUNCTIONS.values().map(|function| &function.module).collect();
    StdlibModule::all().iter().filter(|module| exported.contains(module)).map(|module| module.display_name().to_string()).collect()
}
