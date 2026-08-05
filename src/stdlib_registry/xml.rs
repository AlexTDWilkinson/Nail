//! XML module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("xml_serialize", StdlibFunction {
        rust_path: "std_lib::xml::xml_serialize".to_string(),
        crate_deps: vec![CrateDependency::QuickXml, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeSerialize],
        custom_type_imports: vec![],
        module: StdlibModule::Xml,
        parameters: vec![
            StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
            nail_param!(root_name: s),
        ],
        return_type: nail_type!((s!e)),
        diverging: false,
        description: "Writes a struct, hashmap or array out as XML under the given root element, for the systems that still want it that way.",
        example: "text:s = danger(xml_serialize(invoice, `invoice`));",
    });

    m.insert("xml_deserialize", StdlibFunction {
        rust_path: "std_lib::xml::xml_deserialize".to_string(),
        crate_deps: vec![CrateDependency::QuickXml, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeDeserialize],
        custom_type_imports: vec![],
        module: StdlibModule::Xml,
        parameters: vec![nail_param!(xml_string: s)],
        return_type: nail_type!((T!e)),
        diverging: false,
        description: "Reads XML into a value. The type on the left of the assignment says what to read it as. Struct fields match child elements of the same name.",
        example: "invoice:Invoice = danger(xml_deserialize(text));",
    });
}
