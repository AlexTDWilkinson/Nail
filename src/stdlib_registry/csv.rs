//! Csv module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("csv_parse", StdlibFunction {
        rust_path: "std_lib::csv::parse".to_string(),
        crate_deps: vec![CrateDependency::Csv, CrateDependency::DashMap],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("CSV_Options", "nail::std_lib::csv"), ("CSV_Trim", "nail::std_lib::csv")],
        module: StdlibModule::Csv,
        parameters: vec![
            StdlibParameter { name: "text".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "options".to_string(), param_type: NailDataTypeDescriptor::Struct("CSV_Options".to_string()), pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::HashMap(
            Box::new(NailDataTypeDescriptor::String),
            Box::new(NailDataTypeDescriptor::String),
        ))))),
        diverging: false,
        description: "Parses CSV text into one hashmap per row, keyed by the header row. Quote-aware, so a field containing the delimiter or a newline stays intact.",
        example: "rows:a:h<s,s> = danger(csv_parse(text, csv_default_options()));",
    });

    m.insert("csv_serialize", StdlibFunction {
        rust_path: "std_lib::csv::serialize".to_string(),
        crate_deps: vec![CrateDependency::Csv, CrateDependency::DashMap],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("CSV_Options", "nail::std_lib::csv"), ("CSV_Trim", "nail::std_lib::csv")],
        module: StdlibModule::Csv,
        parameters: vec![
            StdlibParameter { name: "headers".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)), pass_by_reference: false },
            StdlibParameter {
                name: "rows".to_string(),
                param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)))),
                pass_by_reference: false,
            },
            StdlibParameter { name: "options".to_string(), param_type: NailDataTypeDescriptor::Struct("CSV_Options".to_string()), pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
        diverging: false,
        description: "Writes rows out as CSV text, with the columns named and in the order given. Quotes any field holding the delimiter, a quote or a newline, and doubles a quote inside one. A row missing a column is written as an empty field.",
        example: "text:s = danger(csv_serialize([`name`, `city`], rows, csv_default_options()));",
    });

    m.insert("csv_write", StdlibFunction {
        rust_path: "std_lib::csv::write".to_string(),
        crate_deps: vec![CrateDependency::Csv, CrateDependency::DashMap, CrateDependency::Tokio],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("CSV_Options", "nail::std_lib::csv"), ("CSV_Trim", "nail::std_lib::csv")],
        module: StdlibModule::Csv,
        parameters: vec![
            StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "headers".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)), pass_by_reference: false },
            StdlibParameter {
                name: "rows".to_string(),
                param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)))),
                pass_by_reference: false,
            },
            StdlibParameter { name: "options".to_string(), param_type: NailDataTypeDescriptor::Struct("CSV_Options".to_string()), pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        diverging: false,
        description: "Writes rows straight to a file as CSV, with the same escaping as csv_serialize. The file is put in place by a rename, so a reader never catches it half written.",
        example: "danger(csv_write(`export.csv`, [`name`, `city`], rows, csv_default_options()));",
    });

    m.insert("csv_default_options", StdlibFunction {
        rust_path: "std_lib::csv::default_options".to_string(),
        crate_deps: vec![CrateDependency::Csv],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("CSV_Options", "nail::std_lib::csv"), ("CSV_Trim", "nail::std_lib::csv")],
        module: StdlibModule::Csv,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Struct("CSV_Options".to_string()),
        diverging: false,
        description: "The default CSV options: comma separated, double-quoted, with a header row. Nail has no default field values, so this saves spelling out every field of CSV_Options.",
        example: "options:CSV_Options = csv_default_options();",
    });

    m.insert("csv_open", StdlibFunction {
        rust_path: "std_lib::csv::open".to_string(),
        crate_deps: vec![CrateDependency::Csv, CrateDependency::DashMap],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("CSV_Options", "nail::std_lib::csv"), ("CSV_Trim", "nail::std_lib::csv"), ("CSV_Reader", "nail::std_lib::csv")],
        module: StdlibModule::Csv,
        parameters: vec![
            StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "options".to_string(), param_type: NailDataTypeDescriptor::Struct("CSV_Options".to_string()), pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("CSV_Reader".to_string()))),
        diverging: false,
        description: "Opens a CSV file for batch reading, for a file too large to hold in memory. Read from it with csv_next_rows and release it with csv_close.",
        example: "reader:CSV_Reader = danger(csv_open(`big.csv`, csv_default_options()));",
    });

    m.insert("csv_next_rows", StdlibFunction {
        rust_path: "std_lib::csv::next_rows".to_string(),
        crate_deps: vec![CrateDependency::Csv, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("CSV_Reader", "nail::std_lib::csv")],
        module: StdlibModule::Csv,
        parameters: vec![
            StdlibParameter { name: "reader".to_string(), param_type: NailDataTypeDescriptor::Struct("CSV_Reader".to_string()), pass_by_reference: true },
            StdlibParameter { name: "count".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::HashMap(
            Box::new(NailDataTypeDescriptor::String),
            Box::new(NailDataTypeDescriptor::String),
        ))))),
        diverging: false,
        description: "Reads up to `count` more rows from an open reader. A batch shorter than `count` means the file is finished, so callers loop until they get one.",
        example: "batch:a:h<s,s> = danger(csv_next_rows(reader, 10000));",
    });

    m.insert("csv_close", StdlibFunction {
        rust_path: "std_lib::csv::close".to_string(),
        crate_deps: vec![CrateDependency::Csv],
        struct_derives: vec![],
        custom_type_imports: vec![("CSV_Reader", "nail::std_lib::csv")],
        module: StdlibModule::Csv,
        parameters: vec![
            StdlibParameter { name: "reader".to_string(), param_type: NailDataTypeDescriptor::Struct("CSV_Reader".to_string()), pass_by_reference: true },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        diverging: false,
        description: "Closes a reader opened by csv_open and releases its file descriptor. A reader that is never closed holds its descriptor for the life of the process.",
        example: "danger(csv_close(reader));",
    });
}
