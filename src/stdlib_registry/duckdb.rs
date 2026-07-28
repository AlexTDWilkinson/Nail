//! Duckdb module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("db_duckdb_open", StdlibFunction {
        rust_path: "std_lib::duckdb::duckdb_open".to_string(),
        crate_deps: vec![CrateDependency::Duckdb, CrateDependency::DashMap, CrateDependency::Uuid, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_DuckDB", "nail::std_lib::duckdb")],
        module: StdlibModule::Duckdb,
        parameters: vec![
            StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_DuckDB".to_string()))),
        diverging: false,
        description: "Open DuckDB database file",
        example: "db:DB_DuckDB = danger(db_duckdb_open(`analytics.duckdb`));",
    });

m.insert("db_duckdb_memory", StdlibFunction {
        rust_path: "std_lib::duckdb::duckdb_memory".to_string(),
        crate_deps: vec![CrateDependency::Duckdb, CrateDependency::DashMap, CrateDependency::Uuid, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_DuckDB", "nail::std_lib::duckdb")],
        module: StdlibModule::Duckdb,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_DuckDB".to_string()))),
        diverging: false,
        description: "Open in-memory DuckDB database",
        example: "db:DB_DuckDB = danger(db_duckdb_memory());",
    });

m.insert("db_duckdb_execute", StdlibFunction {
        rust_path: "std_lib::duckdb::duckdb_execute".to_string(),
        crate_deps: vec![CrateDependency::Duckdb, CrateDependency::DashMap, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_DuckDB", "nail::std_lib::duckdb"), ("DB_DuckDB_Result", "nail::std_lib::duckdb")],
        module: StdlibModule::Duckdb,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DuckDB".to_string()), pass_by_reference: true },
            StdlibParameter { name: "sql".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_DuckDB_Result".to_string()))),
        diverging: false,
        description: "Execute DuckDB SQL statement (CREATE, INSERT, UPDATE, DELETE, COPY)",
        example: "result:DB_DuckDB_Result = danger(db_duckdb_execute(db, `CREATE TABLE t (id INTEGER)`));",
    });

m.insert("db_duckdb_query", StdlibFunction {
        rust_path: "std_lib::duckdb::duckdb_query".to_string(),
        crate_deps: vec![CrateDependency::Duckdb, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("DB_DuckDB", "nail::std_lib::duckdb")],
        module: StdlibModule::Duckdb,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DuckDB".to_string()), pass_by_reference: true },
            StdlibParameter { name: "sql".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]))))),
        diverging: false,
        description: "Query DuckDB and return results as typed structs",
        example: "rows:a:Person = danger(db_duckdb_query(db, `SELECT name, age FROM people`));",
    });

m.insert("db_duckdb_query_single", StdlibFunction {
        rust_path: "std_lib::duckdb::duckdb_query_single".to_string(),
        crate_deps: vec![CrateDependency::Duckdb, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("DB_DuckDB", "nail::std_lib::duckdb")],
        module: StdlibModule::Duckdb,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DuckDB".to_string()), pass_by_reference: true },
            StdlibParameter { name: "sql".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]))),
        diverging: false,
        description: "Query DuckDB and return single result as typed struct",
        example: "person:Person = danger(db_duckdb_query_single(db, `SELECT name, age FROM people LIMIT 1`));",
    });

m.insert("db_duckdb_close", StdlibFunction {
        rust_path: "std_lib::duckdb::duckdb_close".to_string(),
        crate_deps: vec![CrateDependency::Duckdb, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_DuckDB", "nail::std_lib::duckdb")],
        module: StdlibModule::Duckdb,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DuckDB".to_string()), pass_by_reference: true }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        diverging: false,
        description: "Close DuckDB connection",
        example: "danger(db_duckdb_close(db));",
    });

m.insert("db_duckdb_execute_batch", StdlibFunction {
        rust_path: "std_lib::duckdb::duckdb_execute_batch".to_string(),
        crate_deps: vec![CrateDependency::Duckdb, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_DuckDB", "nail::std_lib::duckdb"), ("DB_DuckDB_Result", "nail::std_lib::duckdb")],
        module: StdlibModule::Duckdb,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_DuckDB".to_string()), pass_by_reference: true },
            StdlibParameter { name: "statements".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)), pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_DuckDB_Result".to_string()))),
        diverging: false,
        description: "Execute multiple DuckDB SQL statements in a single transaction (all succeed or all fail)",
        example: "result:DB_DuckDB_Result = danger(db_duckdb_execute_batch(db, statements));",
    });
}
