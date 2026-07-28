//! Json module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("json_serialize", StdlibFunction {
        rust_path: "std_lib::json::json_serialize".to_string(),
        crate_deps: vec![CrateDependency::SerdeJson, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeSerialize],
        custom_type_imports: vec![],
        module: StdlibModule::Json,
        parameters: vec![
            StdlibParameter {
                name: "value".to_string(),
                param_type: NailDataTypeDescriptor::Any,
                pass_by_reference: false,
            },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
        diverging: false,
        description: "Serialize a value (struct, enum, or array) to a JSON string",
        example: "json_serialize(my_struct)",
    });

m.insert("json_deserialize", StdlibFunction {
        rust_path: "std_lib::json::json_deserialize".to_string(),
        crate_deps: vec![CrateDependency::SerdeJson, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeDeserialize],
        custom_type_imports: vec![],
        module: StdlibModule::Json,
        parameters: vec![
            StdlibParameter {
                name: "json_string".to_string(),
                param_type: NailDataTypeDescriptor::String,
                pass_by_reference: false,
            },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]))),
        diverging: false,
        description: "Deserialize a JSON string to a value (struct, enum, or array)",
        example: "person:Person = danger(json_deserialize(json_string))",
    });
}
