//! TOML module stdlib registry entries.
//!
//! The same two functions as the JSON module, with the same shape: the value
//! going in may be any struct, and the value coming out is whatever type the
//! assignment says to read it as.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("toml_serialize", StdlibFunction {
        rust_path: "std_lib::toml::toml_serialize".to_string(),
        crate_deps: vec![CrateDependency::Toml, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeSerialize],
        custom_type_imports: vec![],
        module: StdlibModule::Toml,
        parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }],
        return_type: nail_type!((s!e)),
        diverging: false,
        description: "Writes a struct, hashmap or array out as TOML - the format a person edits a configuration file in.",
        example: "text:s = danger(toml_serialize(settings));",
    });

    m.insert("toml_deserialize", StdlibFunction {
        rust_path: "std_lib::toml::toml_deserialize".to_string(),
        crate_deps: vec![CrateDependency::Toml, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeDeserialize],
        custom_type_imports: vec![],
        module: StdlibModule::Toml,
        parameters: vec![nail_param!(toml_string: s)],
        return_type: nail_type!((T!e)),
        diverging: false,
        description: "Reads TOML into a value; the type on the left of the assignment says what to read it as, and a document that does not match names the field that did not fit.",
        example: "settings:Settings = danger(toml_deserialize(text));",
    });
}
