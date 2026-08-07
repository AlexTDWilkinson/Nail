//! Postgres module stdlib registry entries.
//!
//! Every signature names a stdlib struct, which `simple_fns!` cannot express, so
//! they are written out in full.

use super::*;

fn postgres_param() -> StdlibParameter {
    return StdlibParameter { name: "db".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_Postgres".to_string()), pass_by_reference: true };
}

fn imports() -> Vec<(&'static str, &'static str)> {
    return vec![("DB_Postgres", "nail::std_lib::postgres"), ("DB_PostgresResult", "nail::std_lib::postgres")];
}

fn deps() -> Vec<CrateDependency> {
    return vec![CrateDependency::TokioPostgres, CrateDependency::Tokio, CrateDependency::SerdeJson, CrateDependency::Serde, CrateDependency::Uuid, CrateDependency::DashMap];
}

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("db_postgres_connect", StdlibFunction {
        rust_path: "std_lib::postgres::connect".to_string(),
        crate_deps: deps(),
        struct_derives: vec![],
        custom_type_imports: imports(),
        module: StdlibModule::Postgres,
        parameters: vec![nail_param!(url: s)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_Postgres".to_string()))),
        diverging: false,
        description: "Connects to a Postgres server with a postgres:// connection string. The connection is not encrypted, so it belongs on localhost or a private network - across the internet, tunnel it.",
        example: "db:DB_Postgres = danger(db_postgres_connect(danger(env_get(`DATABASE_URL`))));",
    });

    m.insert("db_postgres_close", StdlibFunction {
        rust_path: "std_lib::postgres::close".to_string(),
        crate_deps: deps(),
        struct_derives: vec![],
        custom_type_imports: imports(),
        module: StdlibModule::Postgres,
        parameters: vec![postgres_param()],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Closes a connection and forgets its handle. Statements on it afterwards are an error rather than a hang.",
        example: "db:DB_Postgres = danger(db_postgres_connect(`postgres://localhost/app`));\ndanger(db_postgres_close(db));",
    });

    m.insert("db_postgres_execute", StdlibFunction {
        rust_path: "std_lib::postgres::execute".to_string(),
        crate_deps: deps(),
        struct_derives: vec![],
        custom_type_imports: imports(),
        module: StdlibModule::Postgres,
        parameters: vec![postgres_param(), nail_param!(sql: s), nail_param!(params: [s])],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_PostgresResult".to_string()))),
        diverging: false,
        description: "Runs a statement that changes data, binding the values to $1, $2 and so on rather than putting them in the SQL text, and returns how many rows changed.",
        example: "db:DB_Postgres = danger(db_postgres_connect(`postgres://localhost/app`));\nname:s = `Ada`;\nresult:DB_PostgresResult = danger(db_postgres_execute(db, `INSERT INTO people (name) VALUES ($1)`, [name]));",
    });

    m.insert("db_postgres_execute_batch", StdlibFunction {
        rust_path: "std_lib::postgres::execute_batch".to_string(),
        crate_deps: deps(),
        struct_derives: vec![],
        custom_type_imports: imports(),
        module: StdlibModule::Postgres,
        parameters: vec![postgres_param(), nail_param!(statements: s)],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Runs several statements in one round trip, for a schema created on startup. Nothing is bound, so nothing from outside the program belongs in the text.",
        example: "db:DB_Postgres = danger(db_postgres_connect(`postgres://localhost/app`));\nschema:s = `CREATE TABLE people (id SERIAL, name TEXT, age INT)`;\ndanger(db_postgres_execute_batch(db, schema));",
    });

    m.insert("db_postgres_query", StdlibFunction {
        rust_path: "std_lib::postgres::query".to_string(),
        crate_deps: deps(),
        struct_derives: vec![StructDerive::SerdeDeserialize],
        custom_type_imports: imports(),
        module: StdlibModule::Postgres,
        parameters: vec![postgres_param(), nail_param!(sql: s), nail_param!(params: [s])],
        return_type: nail_type!(([T]!e)),
        diverging: false,
        description: "Returns every row of a query as the struct the assignment asks for, binding the values to $1, $2 and so on.",
        example: "struct Person { id:i, name:s }\n\ndb:DB_Postgres = danger(db_postgres_connect(`postgres://localhost/app`));\nminimum:s = `18`;\npeople:a:Person = danger(db_postgres_query(db, `SELECT id, name FROM people WHERE age > $1`, [minimum]));",
    });

    m.insert("db_postgres_query_single", StdlibFunction {
        rust_path: "std_lib::postgres::query_single".to_string(),
        crate_deps: deps(),
        struct_derives: vec![StructDerive::SerdeDeserialize],
        custom_type_imports: imports(),
        module: StdlibModule::Postgres,
        parameters: vec![postgres_param(), nail_param!(sql: s), nail_param!(params: [s])],
        return_type: nail_type!((T!e)),
        diverging: false,
        description: "Returns the one row a query returns. No rows or several rows are both errors, which makes this right for a lookup by key and wrong for a search.",
        example: "struct Person { id:i, name:s }\n\ndb:DB_Postgres = danger(db_postgres_connect(`postgres://localhost/app`));\nwanted_id:s = `1`;\nperson:Person = danger(db_postgres_query_single(db, `SELECT id, name FROM people WHERE id = $1`, [wanted_id]));",
    });
}
