//! Database module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("db_sqlite_open", StdlibFunction {
        rust_path: "std_lib::database::sqlite_open".to_string(),
        crate_deps: vec![CrateDependency::Rusqlite, CrateDependency::DashMap, CrateDependency::Uuid, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_SQLite", "nail::std_lib::database")],
        module: StdlibModule::Database,
        parameters: vec![
            StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_SQLite".to_string()))),
        diverging: false,
        description: "Open SQLite database",
        example: "db:DB_SQLite = danger(db_sqlite_open(`app.db`));",
    });

m.insert("db_sqlite_memory", StdlibFunction {
        rust_path: "std_lib::database::sqlite_memory".to_string(),
        crate_deps: vec![CrateDependency::Rusqlite, CrateDependency::DashMap, CrateDependency::Uuid, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_SQLite", "nail::std_lib::database")],
        module: StdlibModule::Database,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_SQLite".to_string()))),
        diverging: false,
        description: "Open in-memory SQLite database",
        example: "db:DB_SQLite = danger(db_sqlite_memory());",
    });

m.insert("db_sqlite_execute", StdlibFunction {
        rust_path: "std_lib::database::sqlite_execute".to_string(),
        crate_deps: vec![CrateDependency::Rusqlite, CrateDependency::DashMap, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_SQLite", "nail::std_lib::database"), ("DB_Result", "nail::std_lib::database")],
        module: StdlibModule::Database,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_SQLite".to_string()), pass_by_reference: true },
            StdlibParameter { name: "sql".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_Result".to_string()))),
        diverging: false,
        description: "Execute SQL statement",
        example: "result:DB_Result = danger(db_sqlite_execute(db, `CREATE TABLE t (id INTEGER)`));",
    });

m.insert("db_sqlite_query", StdlibFunction {
        rust_path: "std_lib::database::sqlite_query".to_string(),
        crate_deps: vec![CrateDependency::Rusqlite, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("DB_SQLite", "nail::std_lib::database")],
        module: StdlibModule::Database,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_SQLite".to_string()), pass_by_reference: true },
            StdlibParameter { name: "sql".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::TypeVar("T".to_string()))))),
        diverging: false,
        description: "Query database and return results as typed structs",
        example: "rows:a:Person = danger(db_sqlite_query(db, `SELECT name, age FROM people`));",
    });

m.insert("db_sqlite_query_single", StdlibFunction {
        rust_path: "std_lib::database::sqlite_query_single".to_string(),
        crate_deps: vec![CrateDependency::Rusqlite, CrateDependency::Serde],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("DB_SQLite", "nail::std_lib::database")],
        module: StdlibModule::Database,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_SQLite".to_string()), pass_by_reference: true },
            StdlibParameter { name: "sql".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::TypeVar("T".to_string()))),
        diverging: false,
        description: "Query database and return single result as typed struct",
        example: "person:Person = danger(db_sqlite_query_single(db, `SELECT name, age FROM people LIMIT 1`));",
    });

m.insert("db_sqlite_close", StdlibFunction {
        rust_path: "std_lib::database::sqlite_close".to_string(),
        crate_deps: vec![CrateDependency::Rusqlite, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_SQLite", "nail::std_lib::database")],
        module: StdlibModule::Database,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_SQLite".to_string()), pass_by_reference: true }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        diverging: false,
        description: "Close database connection",
        example: "danger(db_sqlite_close(db));",
    });

m.insert("db_sqlite_begin", StdlibFunction {
        rust_path: "std_lib::database::sqlite_begin".to_string(),
        crate_deps: vec![CrateDependency::Rusqlite, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_SQLite", "nail::std_lib::database")],
        module: StdlibModule::Database,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_SQLite".to_string()), pass_by_reference: true }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        diverging: false,
        description: "Begin transaction (prefer db_sqlite_execute_batch for safer transactions)",
        example: "danger(db_sqlite_begin(db));",
    });

m.insert("db_sqlite_commit", StdlibFunction {
        rust_path: "std_lib::database::sqlite_commit".to_string(),
        crate_deps: vec![CrateDependency::Rusqlite, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_SQLite", "nail::std_lib::database")],
        module: StdlibModule::Database,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_SQLite".to_string()), pass_by_reference: true }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        diverging: false,
        description: "Commit transaction (prefer db_sqlite_execute_batch for safer transactions)",
        example: "danger(db_sqlite_commit(db));",
    });

m.insert("db_sqlite_rollback", StdlibFunction {
        rust_path: "std_lib::database::sqlite_rollback".to_string(),
        crate_deps: vec![CrateDependency::Rusqlite, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_SQLite", "nail::std_lib::database")],
        module: StdlibModule::Database,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_SQLite".to_string()), pass_by_reference: true }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        diverging: false,
        description: "Rollback transaction (prefer db_sqlite_execute_batch for safer transactions)",
        example: "danger(db_sqlite_rollback(db));",
    });

m.insert("db_sqlite_execute_batch", StdlibFunction {
        rust_path: "std_lib::database::sqlite_execute_batch".to_string(),
        crate_deps: vec![CrateDependency::Rusqlite, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_SQLite", "nail::std_lib::database"), ("DB_Result", "nail::std_lib::database")],
        module: StdlibModule::Database,
        parameters: vec![
            StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_SQLite".to_string()), pass_by_reference: true },
            StdlibParameter { name: "statements".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)), pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_Result".to_string()))),
        diverging: false,
        description: "Execute multiple SQL statements in a single transaction (all succeed or all fail)",
        example: "result:DB_Result = danger(db_sqlite_execute_batch(db, statements));",
    });
}
