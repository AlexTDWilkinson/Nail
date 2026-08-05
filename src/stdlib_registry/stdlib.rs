//! Stdlib module registry entries: the standard library described as data.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert(
        "stdlib_functions",
        StdlibFunction {
            rust_path: "std_lib::stdlib::functions".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![("STDLIB_Function", "nail::std_lib::stdlib")],
            module: StdlibModule::Stdlib,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("STDLIB_Function".to_string()))),

            diverging: false,
            description: "Every function the standard library provides, as data: name, module, signature, description and example. Sorted by module, then by name. The list comes from the same registry the type checker uses, so it is exactly what this compiler can call.",
            example: "functions:a:STDLIB_Function = stdlib_functions();",
        },
    );

    simple_fns! { m, Stdlib:
        "stdlib_modules" => "std_lib::stdlib::modules", () -> [s],
            "The standard library's namespaces, spelled the way calls spell them (db, string, net), in the order stdlib_functions lists their functions.",
            "modules:a:s = stdlib_modules();";
    }
}
