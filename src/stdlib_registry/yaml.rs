//! YAML module stdlib registry entries.
//!
//! The same two functions as the TOML module, for the documents a program does
//! not get to choose the format of.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("yaml_serialize", StdlibFunction {
        rust_path: "std_lib::yaml::yaml_serialize".to_string(),
        crate_deps: vec![CrateDependency::SerdeYaml, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeSerialize],
        custom_type_imports: vec![],
        module: StdlibModule::Yaml,
        parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }],
        return_type: nail_type!((s!e)),
        diverging: false,
        description: "Writes a struct, hashmap or array out as YAML - the format CI files, manifests and compose files are written in.",
        example: "text:s = danger(yaml_serialize(job));",
    });

    m.insert("yaml_deserialize", StdlibFunction {
        rust_path: "std_lib::yaml::yaml_deserialize".to_string(),
        crate_deps: vec![CrateDependency::SerdeYaml, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeDeserialize],
        custom_type_imports: vec![],
        module: StdlibModule::Yaml,
        parameters: vec![nail_param!(yaml_string: s)],
        return_type: nail_type!((T!e)),
        diverging: false,
        description: "Reads YAML into a value; the type on the left of the assignment says what to read it as, and a document that does not match names the field that did not fit.",
        example: "job:Job = danger(yaml_deserialize(text));",
    });
}
