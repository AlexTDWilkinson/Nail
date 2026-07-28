//! DataFusion module stdlib registry entries - SQL analytics over
//! parquet/CSV files and in-memory tables, pure Rust (no C++ dependency).

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("db_datafusion_session", StdlibFunction {
        rust_path: "std_lib::datafusion::datafusion_session".to_string(),
        crate_deps: vec![CrateDependency::DataFusion, CrateDependency::DashMap, CrateDependency::Uuid, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_DataFusion", "nail::std_lib::datafusion")],
        module: StdlibModule::DataFusion,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_DataFusion".to_string()))),
        diverging: false,
        description: "Open an in-memory DataFusion analytics session",
        example: "db:DB_DataFusion = danger(db_datafusion_session());",
    });

    m.insert("db_datafusion_register_parquet", StdlibFunction {
        rust_path: "std_lib::datafusion::datafusion_register_parquet".to_string(),
        crate_deps: vec![CrateDependency::DataFusion, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_DataFusion", "nail::std_lib::datafusion")],
        module: StdlibModule::DataFusion,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DataFusion".to_string()), pass_by_reference: true },
            StdlibParameter { name: "table".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        diverging: false,
        description: "Register a Parquet file as a queryable SQL table",
        example: "danger(db_datafusion_register_parquet(db, `homes`, `homes.parquet`));",
    });

    m.insert("db_datafusion_register_csv", StdlibFunction {
        rust_path: "std_lib::datafusion::datafusion_register_csv".to_string(),
        crate_deps: vec![CrateDependency::DataFusion, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_DataFusion", "nail::std_lib::datafusion")],
        module: StdlibModule::DataFusion,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DataFusion".to_string()), pass_by_reference: true },
            StdlibParameter { name: "table".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        diverging: false,
        description: "Register a CSV file as a queryable SQL table",
        example: "danger(db_datafusion_register_csv(db, `sales`, `sales.csv`));",
    });

    m.insert("db_datafusion_execute", StdlibFunction {
        rust_path: "std_lib::datafusion::datafusion_execute".to_string(),
        crate_deps: vec![CrateDependency::DataFusion, CrateDependency::DashMap, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_DataFusion", "nail::std_lib::datafusion"), ("DB_DataFusion_Result", "nail::std_lib::datafusion")],
        module: StdlibModule::DataFusion,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DataFusion".to_string()), pass_by_reference: true },
            StdlibParameter { name: "sql".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_DataFusion_Result".to_string()))),
        diverging: false,
        description: "Execute a DataFusion SQL statement (CREATE TABLE, INSERT)",
        example: "result:DB_DataFusion_Result = danger(db_datafusion_execute(db, `CREATE TABLE t (id INT)`));",
    });

    m.insert("db_datafusion_query", StdlibFunction {
        rust_path: "std_lib::datafusion::datafusion_query".to_string(),
        crate_deps: vec![CrateDependency::DataFusion, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("DB_DataFusion", "nail::std_lib::datafusion")],
        module: StdlibModule::DataFusion,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DataFusion".to_string()), pass_by_reference: true },
            StdlibParameter { name: "sql".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]))))),
        diverging: false,
        description: "Query DataFusion with SQL and return results as typed structs",
        example: "rows:a:Person = danger(db_datafusion_query(db, `SELECT name, age FROM people`));",
    });

    m.insert("db_datafusion_query_single", StdlibFunction {
        rust_path: "std_lib::datafusion::datafusion_query_single".to_string(),
        crate_deps: vec![CrateDependency::DataFusion, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("DB_DataFusion", "nail::std_lib::datafusion")],
        module: StdlibModule::DataFusion,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DataFusion".to_string()), pass_by_reference: true },
            StdlibParameter { name: "sql".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]))),
        diverging: false,
        description: "Query DataFusion with SQL and return a single result as a typed struct",
        example: "person:Person = danger(db_datafusion_query_single(db, `SELECT name, age FROM people LIMIT 1`));",
    });

    m.insert("db_datafusion_close", StdlibFunction {
        rust_path: "std_lib::datafusion::datafusion_close".to_string(),
        crate_deps: vec![CrateDependency::DataFusion, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_DataFusion", "nail::std_lib::datafusion")],
        module: StdlibModule::DataFusion,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DataFusion".to_string()), pass_by_reference: true }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        diverging: false,
        description: "Close a DataFusion session",
        example: "danger(db_datafusion_close(db));",
    });
}
