//! Json module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    // Reading a few fields straight out of a document, for the answers from
    // somebody else's API that nobody wants to describe as a struct first.
    simple_fns! { m, Json:
        "json_get_string" [SerdeJson] => "std_lib::json::get_string", (json: s, path: s) -> (s!e),
            "Returns the text at a dotted path like user.name, where a number in the path indexes a list; a number or boolean there comes back as it was written. Errors if the path is not there.",
            "name:s = danger(json_get_string(body, `user.name`));";
        "json_get_int" [SerdeJson] => "std_lib::json::get_int", (json: s, path: s) -> (i!e),
            "Returns the whole number at a dotted path; a fraction is an error rather than a silent rounding.",
            "age:i = danger(json_get_int(body, `user.age`));";
        "json_get_float" [SerdeJson] => "std_lib::json::get_float", (json: s, path: s) -> (f!e),
            "Returns the number at a dotted path, whole or fractional.",
            "score:f = danger(json_get_float(body, `user.score`));";
        "json_get_bool" [SerdeJson] => "std_lib::json::get_bool", (json: s, path: s) -> (b!e),
            "Returns the true or false at a dotted path; the strings true and false count too.",
            "active:b = danger(json_get_bool(body, `user.active`));";
        "json_has" [SerdeJson] => "std_lib::json::has", (json: s, path: s) -> b,
            "Whether there is anything at a dotted path. A field that is present but null counts as missing, and text that is not JSON is simply false.",
            "present:b = json_has(body, `user.email`);";
        "json_array_length" [SerdeJson] => "std_lib::json::array_length", (json: s, path: s) -> (i!e),
            "Returns how many items the list at a dotted path holds - the number to count up to when reading them one at a time. An empty path asks about the whole document.",
            "total:i = danger(json_array_length(body, `items`));";
        "json_pretty" [SerdeJson] => "std_lib::json::pretty", (json: s) -> (s!e),
            "Returns the same JSON indented, for a file a person will read or a diff that shows which field changed.",
            "readable:s = danger(json_pretty(body));";
        "json_compact" [SerdeJson] => "std_lib::json::compact", (json: s) -> (s!e),
            "Returns the same JSON with every space between values taken out - the form to send or store.",
            "wire:s = danger(json_compact(body));";
    }

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
